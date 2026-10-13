//! Theme registry: embedded built-ins + user directory, alias resolution and
//! nearest-match suggestions.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::builtin::BUILTIN_THEMES;
use super::file::{
    is_valid_id, resolve, write_suggestions, RawTheme, ThemeError, ThemeFileError,
    MAX_THEME_FILE_BYTES,
};
use super::{Theme, ThemeSource};

/// Bound on how many user theme files are considered.
const MAX_USER_THEME_FILES: usize = 256;
/// Bound on `extends` chain length.
const MAX_EXTENDS_DEPTH: usize = 8;
const MAX_SUGGESTIONS: usize = 3;

/// One user theme file's content, read from disk or supplied in memory.
#[derive(Debug, Clone)]
pub struct UserThemeSource {
    /// File stem; must equal the theme `id`.
    pub stem: String,
    pub path: PathBuf,
    pub text: Result<String, ThemeError>,
}

/// A name that resolves to no theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownTheme {
    pub name: String,
    pub suggestions: Vec<String>,
    /// Set when a user theme file with this id exists but failed validation.
    pub invalid_file: Option<ThemeFileError>,
}

impl fmt::Display for UnknownTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown theme `{}`", self.name)?;
        write_suggestions(f, &self.suggestions)?;
        if let Some(invalid) = &self.invalid_file {
            write!(f, " ({invalid})")?;
        }
        Ok(())
    }
}

impl std::error::Error for UnknownTheme {}

/// Resolved set of themes.
#[derive(Debug, Clone)]
pub struct Registry {
    /// Sorted by id.
    themes: Vec<Arc<Theme>>,
    /// alias -> id (only aliases that do not collide with an id).
    aliases: BTreeMap<String, String>,
    diagnostics: Vec<ThemeFileError>,
}

impl Registry {
    /// Built-in themes only (no filesystem access).
    #[must_use]
    pub fn builtin() -> Self {
        Self::with_user_sources(Vec::new())
    }

    /// Built-ins plus every `*.toml` in `dir`. A missing directory is not an
    /// error; unreadable or invalid files are reported in
    /// [`Self::diagnostics`] and skipped.
    #[must_use]
    pub fn load_dir(dir: &Path) -> Self {
        let (sources, mut diagnostics) = read_user_dir(dir);
        let mut registry = Self::with_user_sources(sources);
        diagnostics.append(&mut registry.diagnostics);
        registry.diagnostics = diagnostics;
        registry
    }

    /// Built-ins plus the given user theme sources (used by tests and by
    /// `sunreactorctl theme validate <file>`).
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn with_user_sources(sources: Vec<UserThemeSource>) -> Self {
        let mut diagnostics = Vec::new();

        let mut builtin_raw: BTreeMap<String, RawTheme> = BTreeMap::new();
        let mut themes: BTreeMap<String, Arc<Theme>> = BTreeMap::new();
        for (stem, text) in BUILTIN_THEMES {
            let resolved = RawTheme::parse(text)
                .map_err(|error| vec![error])
                .and_then(|raw| {
                    resolve(&raw, Some(stem), ThemeSource::Builtin, None).map(|theme| (raw, theme))
                });
            match resolved {
                Ok((raw, theme)) => {
                    builtin_raw.insert((*stem).to_owned(), raw);
                    themes.insert(theme.id.clone(), Arc::new(theme));
                }
                Err(errors) => diagnostics.push(ThemeFileError {
                    path: None,
                    stem: (*stem).to_owned(),
                    errors,
                }),
            }
        }
        let mut aliases: BTreeMap<String, String> = BTreeMap::new();
        for theme in themes.values() {
            for alias in &theme.aliases {
                aliases.insert(alias.clone(), theme.id.clone());
            }
        }

        // Parse user files; a parse/read failure removes the file from play.
        let mut user_raw: BTreeMap<String, (PathBuf, RawTheme)> = BTreeMap::new();
        let mut user_paths: HashMap<String, PathBuf> = HashMap::new();
        for source in sources.into_iter().take(MAX_USER_THEME_FILES) {
            let parsed = source.text.and_then(|text| RawTheme::parse(&text));
            match parsed {
                Ok(raw) => {
                    user_paths.insert(source.stem.clone(), source.path.clone());
                    user_raw.insert(source.stem, (source.path, raw));
                }
                Err(error) => diagnostics.push(ThemeFileError {
                    path: Some(source.path),
                    stem: source.stem,
                    errors: vec![error],
                }),
            }
        }

        let lookup = RawLookup {
            builtin: &builtin_raw,
            builtin_aliases: &aliases,
            user: &user_raw,
        };
        let mut user_themes: Vec<Arc<Theme>> = Vec::new();
        for (stem, (path, raw)) in &user_raw {
            let result = lookup
                .flatten(stem, raw, &mut vec![stem.clone()])
                .and_then(|merged| {
                    resolve(
                        &merged,
                        Some(stem),
                        ThemeSource::User(path.clone()),
                        raw.extends.clone(),
                    )
                });
            match result {
                Ok(theme) => user_themes.push(Arc::new(theme)),
                Err(errors) => diagnostics.push(ThemeFileError {
                    path: Some(path.clone()),
                    stem: stem.clone(),
                    errors,
                }),
            }
        }

        // User themes shadow built-ins with the same id.
        for theme in &user_themes {
            themes.insert(theme.id.clone(), Arc::clone(theme));
        }
        // An id always wins over an alias of the same spelling.
        aliases.retain(|alias, _| !themes.contains_key(alias));
        // User aliases must not collide with anything else.
        for theme in &user_themes {
            let mut conflicts = Vec::new();
            for alias in &theme.aliases {
                if let Some(owner) = themes.get(alias).map(|owner| owner.id.clone()) {
                    conflicts.push(ThemeError::AliasConflict {
                        alias: alias.clone(),
                        owner,
                    });
                } else if let Some(owner) = aliases.get(alias).filter(|owner| **owner != theme.id) {
                    conflicts.push(ThemeError::AliasConflict {
                        alias: alias.clone(),
                        owner: owner.clone(),
                    });
                }
            }
            if conflicts.is_empty() {
                for alias in &theme.aliases {
                    aliases.insert(alias.clone(), theme.id.clone());
                }
            } else {
                diagnostics.push(ThemeFileError {
                    path: user_paths.get(&theme.id).cloned(),
                    stem: theme.id.clone(),
                    errors: conflicts,
                });
            }
        }

        Self {
            themes: themes.into_values().collect(),
            aliases,
            diagnostics,
        }
    }

    /// All themes, sorted by id.
    #[must_use]
    pub fn themes(&self) -> &[Arc<Theme>] {
        &self.themes
    }

    /// Invalid/unreadable theme files seen while loading.
    #[must_use]
    pub fn diagnostics(&self) -> &[ThemeFileError] {
        &self.diagnostics
    }

    /// Exact id lookup.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&Arc<Theme>> {
        self.themes
            .binary_search_by(|theme| theme.id.as_str().cmp(id))
            .ok()
            .map(|index| &self.themes[index])
    }

    /// Resolves an id or alias. Matching is case-insensitive, treats `-` and
    /// spaces as `_`, and finally ignores `_` entirely (`vfdhifi` == `vfd_hifi`)
    /// when that is unambiguous.
    pub fn resolve(&self, name: &str) -> Result<&Arc<Theme>, UnknownTheme> {
        // Fast path: canonical ids (what configs hold after loading) need no
        // normalisation; renderers call this every frame.
        if let Some(theme) = self.get(name) {
            return Ok(theme);
        }
        let key = normalize(name);
        if let Some(theme) = self.get(&key) {
            return Ok(theme);
        }
        if let Some(theme) = self.aliases.get(&key).and_then(|id| self.get(id)) {
            return Ok(theme);
        }
        let compact = compact(&key);
        if !compact.is_empty() {
            let mut matches = self
                .names()
                .filter(|(candidate, _)| self::compact(candidate) == compact)
                .map(|(_, id)| id)
                .collect::<Vec<_>>();
            matches.sort_unstable();
            matches.dedup();
            if let [id] = matches.as_slice() {
                if let Some(theme) = self.get(id) {
                    return Ok(theme);
                }
            }
        }
        Err(UnknownTheme {
            name: name.to_owned(),
            suggestions: self.suggestions(name),
            invalid_file: self
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.stem == key)
                .cloned(),
        })
    }

    /// Resolves `name` for a consumer that must keep running (the daemon,
    /// renderers): never fails. Returns the theme to use plus a warning when
    /// the configured theme could not be used as-is:
    /// - a user theme file with that id is invalid: the built-in with the same
    ///   id is used if there is one (the registry already skipped the file),
    ///   otherwise the default theme;
    /// - the name is unknown (typo, deleted file): the default theme.
    #[must_use]
    pub fn resolve_or_fallback(&self, name: &str) -> (Arc<Theme>, Option<String>) {
        match self.resolve(name) {
            Ok(theme) => {
                let warning = (theme.source == ThemeSource::Builtin)
                    .then(|| {
                        self.diagnostics
                            .iter()
                            .find(|diagnostic| diagnostic.stem == theme.id)
                    })
                    .flatten()
                    .map(|diagnostic| {
                        format!(
                            "{diagnostic}\nusing the built-in `{}` theme instead",
                            theme.id
                        )
                    });
                (Arc::clone(theme), warning)
            }
            Err(unknown) => {
                let fallback = self
                    .get(super::DEFAULT_THEME_ID)
                    .map(Arc::clone)
                    .expect("the default built-in theme always exists");
                let warning = format!(
                    "{unknown}; using the default `{}` theme instead",
                    super::DEFAULT_THEME_ID
                );
                (fallback, Some(warning))
            }
        }
    }

    /// Canonical id for `name`, if it resolves.
    #[must_use]
    pub fn canonical_id(&self, name: &str) -> Option<&str> {
        self.resolve(name).ok().map(|theme| theme.id.as_str())
    }

    /// Up to three theme ids closest to `name` (edit distance over ids and
    /// aliases, plus substring matches).
    #[must_use]
    pub fn suggestions(&self, name: &str) -> Vec<String> {
        let key = normalize(name);
        if key.is_empty() {
            return Vec::new();
        }
        let threshold = (key.chars().count() / 3).max(2);
        let mut scored: Vec<(usize, &str)> = Vec::new();
        for (candidate, id) in self.names() {
            let distance = levenshtein(&key, candidate);
            let contains = key.len() >= 3 && (candidate.contains(&key) || key.contains(candidate));
            let score = if contains { distance.min(1) } else { distance };
            if score <= threshold || contains {
                scored.push((score, id));
            }
        }
        scored.sort_unstable();
        let mut out: Vec<String> = Vec::new();
        for (_, id) in scored {
            if !out.iter().any(|existing| existing == id) {
                out.push(id.to_owned());
            }
            if out.len() == MAX_SUGGESTIONS {
                break;
            }
        }
        out
    }

    /// `(name, owning id)` for every id and alias.
    fn names(&self) -> impl Iterator<Item = (&str, &str)> {
        self.themes
            .iter()
            .map(|theme| (theme.id.as_str(), theme.id.as_str()))
            .chain(
                self.aliases
                    .iter()
                    .map(|(alias, id)| (alias.as_str(), id.as_str())),
            )
    }
}

struct RawLookup<'a> {
    builtin: &'a BTreeMap<String, RawTheme>,
    builtin_aliases: &'a BTreeMap<String, String>,
    user: &'a BTreeMap<String, (PathBuf, RawTheme)>,
}

impl RawLookup<'_> {
    /// Applies the `extends` chain of `raw` (a user theme with id `stem`).
    fn flatten(
        &self,
        stem: &str,
        raw: &RawTheme,
        chain: &mut Vec<String>,
    ) -> Result<RawTheme, Vec<ThemeError>> {
        let Some(base_name) = raw.extends.as_deref() else {
            return Ok(raw.clone());
        };
        if chain.len() > MAX_EXTENDS_DEPTH {
            chain.push(base_name.to_owned());
            return Err(vec![ThemeError::ExtendsCycle {
                chain: chain.clone(),
            }]);
        }
        let key = normalize(base_name);
        // `extends` equal to the theme's own id means "the built-in I shadow".
        let user_base = (key != stem).then(|| self.user.get(&key)).flatten();
        if let Some((_, base_raw)) = user_base {
            if chain.contains(&key) {
                chain.push(key);
                return Err(vec![ThemeError::ExtendsCycle {
                    chain: chain.clone(),
                }]);
            }
            chain.push(key.clone());
            let base = self.flatten(&key, base_raw, chain)?;
            return Ok(raw.clone().merged_over(&base));
        }
        let builtin_id = if self.builtin.contains_key(&key) {
            Some(key.as_str())
        } else {
            self.builtin_aliases.get(&key).map(String::as_str)
        };
        if let Some(base) = builtin_id.and_then(|id| self.builtin.get(id)) {
            return Ok(raw.clone().merged_over(base));
        }
        let mut suggestions = Registry::builtin().suggestions(base_name);
        for id in self.user.keys() {
            if id != stem && levenshtein(&key, id) <= 2 && !suggestions.contains(id) {
                suggestions.push(id.clone());
            }
        }
        suggestions.truncate(MAX_SUGGESTIONS);
        Err(vec![ThemeError::UnknownExtends {
            extends: base_name.to_owned(),
            suggestions,
        }])
    }
}

/// Validates one theme file in the context of the built-ins and (optionally)
/// a user theme directory, so `extends` and alias conflicts are checked the
/// same way the registry would check them. A file with the same stem in
/// `user_dir` is replaced by `path`.
pub fn validate_file(path: &Path, user_dir: Option<&Path>) -> Result<Theme, ThemeFileError> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned();
    if !is_valid_id(&stem) {
        return Err(ThemeFileError {
            path: Some(path.to_path_buf()),
            stem: stem.clone(),
            errors: vec![ThemeError::InvalidId {
                field: String::from("file name"),
                value: stem,
            }],
        });
    }
    let mut sources = user_dir.map(|dir| read_user_dir(dir).0).unwrap_or_default();
    sources.retain(|source| source.stem != stem);
    sources.push(UserThemeSource {
        stem: stem.clone(),
        path: path.to_path_buf(),
        text: read_theme_file(path),
    });
    let registry = Registry::with_user_sources(sources);
    if let Some(error) = registry
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.stem == stem)
    {
        return Err(error.clone());
    }
    registry
        .get(&stem)
        .filter(|theme| matches!(theme.source, ThemeSource::User(_)))
        .map(|theme| Theme::clone(theme))
        .ok_or_else(|| ThemeFileError {
            path: Some(path.to_path_buf()),
            stem,
            errors: vec![ThemeError::Parse {
                message: String::from("theme could not be resolved"),
            }],
        })
}

fn read_user_dir(dir: &Path) -> (Vec<UserThemeSource>, Vec<ThemeFileError>) {
    let mut sources = Vec::new();
    let mut diagnostics = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        // Missing or unreadable directory: no user themes.
        return (sources, diagnostics);
    };
    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();
    for path in paths.into_iter().take(MAX_USER_THEME_FILES) {
        let Some(stem) = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        if !is_valid_id(&stem) {
            diagnostics.push(ThemeFileError {
                path: Some(path.clone()),
                stem: stem.clone(),
                errors: vec![ThemeError::InvalidId {
                    field: String::from("file name"),
                    value: stem,
                }],
            });
            continue;
        }
        let text = read_theme_file(&path);
        sources.push(UserThemeSource { stem, path, text });
    }
    (sources, diagnostics)
}

/// Reads one theme file with the size bound applied.
pub fn read_theme_file(path: &Path) -> Result<String, ThemeError> {
    use std::io::Read;

    let file = std::fs::File::open(path).map_err(|error| ThemeError::Io {
        message: error.to_string(),
    })?;
    let metadata = file.metadata().map_err(|error| ThemeError::Io {
        message: error.to_string(),
    })?;
    if !metadata.is_file() {
        return Err(ThemeError::Io {
            message: String::from("not a regular file"),
        });
    }
    if metadata.len() > MAX_THEME_FILE_BYTES {
        return Err(ThemeError::TooLarge {
            bytes: metadata.len(),
            limit: MAX_THEME_FILE_BYTES,
        });
    }
    let mut text = String::new();
    file.take(MAX_THEME_FILE_BYTES + 1)
        .read_to_string(&mut text)
        .map_err(|error| ThemeError::Io {
            message: error.to_string(),
        })?;
    if text.len() as u64 > MAX_THEME_FILE_BYTES {
        return Err(ThemeError::TooLarge {
            bytes: text.len() as u64,
            limit: MAX_THEME_FILE_BYTES,
        });
    }
    Ok(text)
}

fn normalize(name: &str) -> String {
    name.trim()
        .chars()
        .map(|ch| match ch {
            '-' | ' ' => '_',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

fn compact(name: &str) -> String {
    name.chars().filter(|ch| *ch != '_').collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b_chars: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b_chars.len()).collect();
    let mut current = vec![0; b_chars.len() + 1];
    for (i, a_char) in a.chars().enumerate() {
        current[0] = i + 1;
        for (j, b_char) in b_chars.iter().enumerate() {
            let cost = usize::from(a_char != *b_char);
            current[j + 1] = (previous[j] + cost)
                .min(previous[j + 1] + 1)
                .min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[b_chars.len()]
}

#[cfg(test)]
pub(super) fn levenshtein_for_tests(a: &str, b: &str) -> usize {
    levenshtein(a, b)
}
