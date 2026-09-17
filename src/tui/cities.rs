use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Clone)]
pub struct CityRaw {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "c")]
    pub country: String,
    #[serde(rename = "tz")]
    pub timezone: String,
    #[serde(rename = "la")]
    pub lat: f64,
    #[serde(rename = "lo")]
    pub lon: f64,
}

#[derive(Debug, Clone)]
pub struct City {
    pub name: String,
    pub country: String,
    pub timezone: String,
    pub lat: f64,
    pub lon: f64,
    pub search_key: String,
    /// Normalized words separated by single spaces, for word-prefix matches.
    pub word_key: String,
}

pub fn normalize_for_search(input: &str) -> String {
    let ascii_transliterated = deunicode::deunicode(input);

    ascii_transliterated
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
}

/// Like [`normalize_for_search`], but keeps word boundaries as single spaces.
fn normalize_words(input: &str) -> String {
    let ascii_transliterated = deunicode::deunicode(input);
    let mut words = String::with_capacity(ascii_transliterated.len() + 1);
    words.push(' ');
    for character in ascii_transliterated.chars() {
        if character.is_ascii_alphanumeric() {
            words.extend(character.to_lowercase());
        } else if !words.ends_with(' ') {
            words.push(' ');
        }
    }
    words
}

pub fn get_cities() -> &'static [City] {
    static CITIES: OnceLock<Vec<City>> = OnceLock::new();
    CITIES.get_or_init(|| {
        let data = include_str!("cities.json");
        let raw_cities: Vec<CityRaw> = serde_json::from_str(data).unwrap_or_default();
        raw_cities
            .into_iter()
            .map(|raw| City {
                search_key: normalize_for_search(&raw.name),
                word_key: normalize_words(&raw.name),
                name: raw.name,
                country: raw.country,
                timezone: raw.timezone,
                lat: raw.lat,
                lon: raw.lon,
            })
            .collect()
    })
}

pub fn search_cities(query: &str) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }

    let normalized_query = normalize_for_search(query);
    if normalized_query.is_empty() {
        return Vec::new();
    }

    // Rank what the user most likely means: the exact name, then names that
    // start with the query, then any word that starts with it, then any
    // substring. Shorter names win ties, so "Ist" finds Istanbul first.
    let word_query = format!(" {normalized_query}");
    let mut ranked: Vec<(u8, usize, usize)> = get_cities()
        .iter()
        .enumerate()
        .filter_map(|(index, city)| {
            let rank = if city.search_key == normalized_query {
                0
            } else if city.search_key.starts_with(&normalized_query) {
                1
            } else if city.word_key.contains(&word_query) {
                2
            } else if city.search_key.contains(&normalized_query) {
                3
            } else {
                return None;
            };
            Some((rank, city.search_key.len(), index))
        })
        .collect();
    ranked.sort_unstable();
    ranked
        .into_iter()
        .take(10)
        .map(|(_, _, index)| index)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{get_cities, search_cities};

    fn first_match(query: &str) -> String {
        let index = *search_cities(query).first().expect("a match");
        let city = &get_cities()[index];
        format!("{}, {}", city.name, city.country)
    }

    #[test]
    fn prefix_matches_outrank_substrings() {
        let names: Vec<String> = search_cities("Ist")
            .into_iter()
            .map(|index| get_cities()[index].name.clone())
            .collect();
        assert!(names.iter().any(|name| name == "Istanbul"), "{names:?}");
        let prefix: Vec<bool> = names
            .iter()
            .map(|name| crate::tui::cities::normalize_for_search(name).starts_with("ist"))
            .collect();
        assert!(prefix[0], "{names:?}");
        // Every prefix match is listed before any non-prefix match.
        assert!(
            prefix.windows(2).all(|pair| pair[0] || !pair[1]),
            "{names:?}"
        );
        assert_eq!(first_match("tokyo"), "Tokyo, JP");
    }

    #[test]
    fn transliterated_and_word_prefix_queries_still_match() {
        assert!(search_cities("İstanbul")
            .iter()
            .any(|index| get_cities()[*index].name == "Istanbul"));
        assert!(search_cities("york")
            .iter()
            .any(|index| get_cities()[*index].name == "New York City"));
        assert!(search_cities("").is_empty());
    }
}
