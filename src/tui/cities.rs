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
}

pub fn normalize_for_search(input: &str) -> String {
    let ascii_transliterated = deunicode::deunicode(input);

    ascii_transliterated
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect()
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

    let mut results = Vec::new();
    for (i, city) in get_cities().iter().enumerate() {
        if city.search_key.contains(&normalized_query) {
            results.push(i);
            if results.len() >= 10 {
                break;
            }
        }
    }
    results
}
