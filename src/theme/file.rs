//! Theme file format: raw TOML layer, `extends` merge, strict validation
//! and resolution into a [`Theme`].

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{
    derived_shades, Border, Density, Dither, Family, Font, GlyphStyle, Module, Palette, Renderer,
    Rgb, Theme, ThemeSource, TuiStyle, WidgetStyle, MAX_PIXEL_SCALE, MAX_RADIUS, MAX_SHADES,
    MIN_SHADES,
};

pub(super) const MAX_THEME_FILE_BYTES: u64 = 64 * 1024;
const MAX_NAME_CHARS: usize = 64;
const MAX_ALIASES: usize = 16;
const MAX_ID_CHARS: usize = 48;

/// One validation problem in a theme file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeError {
    /// The file is not valid TOML or has an unknown key / wrong value type.
    Parse { message: String },
    /// The file could not be read.
    Io { message: String },
    /// The file exceeds the size limit.
    TooLarge { bytes: u64, limit: u64 },
    /// A required top-level key is missing (`id`, `name`, `family`).
    MissingField { field: &'static str },
    /// `id` (or an alias) is not `[a-z0-9_]+`.
    InvalidId { field: String, value: String },
    /// `id` differs from the file stem.
    IdMismatch { id: String, file_stem: String },
    /// A colour is not `#rrggbb`.
    BadHex { field: String, value: String },
    /// `family` is not pixel | instrument | terminal.
    UnknownFamily { value: String },
    /// A `[palette]` key is missing (after `extends` is applied).
    MissingPaletteKey { key: &'static str },
    /// Any other out-of-range or unknown value.
    InvalidValue {
        field: String,
        value: String,
        expected: String,
    },
    /// `extends` names a theme that does not exist.
    UnknownExtends {
        extends: String,
        suggestions: Vec<String>,
    },
    /// `extends` chain loops or is too deep.
    ExtendsCycle { chain: Vec<String> },
    /// An alias is already used by another theme's id or alias.
    AliasConflict { alias: String, owner: String },
}

impl fmt::Display for ThemeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse { message } => write!(f, "invalid TOML: {message}"),
            Self::Io { message } => write!(f, "cannot read file: {message}"),
            Self::TooLarge { bytes, limit } => {
                write!(f, "file is {bytes} bytes; theme files are limited to {limit} bytes")
            }
            Self::MissingField { field } => write!(f, "{field}: required key is missing"),
            Self::InvalidId { field, value } => {
                write!(f, "{field}: `{value}` must match [a-z0-9_]+ (max {MAX_ID_CHARS} chars)")
            }
            Self::IdMismatch { id, file_stem } => write!(
                f,
                "id: `{id}` must equal the file name stem `{file_stem}` (rename the file to {id}.toml or change id)"
            ),
            Self::BadHex { field, value } => {
                write!(f, "{field}: `{value}` is not a #rrggbb colour")
            }
            Self::UnknownFamily { value } => write!(
                f,
                "family: unknown family `{value}`; expected {}",
                Family::expected()
            ),
            Self::MissingPaletteKey { key } => {
                write!(f, "palette.{key}: required colour is missing")
            }
            Self::InvalidValue {
                field,
                value,
                expected,
            } => write!(f, "{field}: `{value}` is invalid; expected {expected}"),
            Self::UnknownExtends {
                extends,
                suggestions,
            } => {
                write!(f, "extends: unknown theme `{extends}`")?;
                write_suggestions(f, suggestions)
            }
            Self::ExtendsCycle { chain } => {
                write!(f, "extends: cycle or chain too deep: {}", chain.join(" -> "))
            }
            Self::AliasConflict { alias, owner } => {
                write!(f, "aliases: `{alias}` is already used by theme `{owner}`")
            }
        }
    }
}

pub(super) fn write_suggestions(f: &mut fmt::Formatter<'_>, suggestions: &[String]) -> fmt::Result {
    if suggestions.is_empty() {
        return Ok(());
    }
    let quoted = suggestions
        .iter()
        .map(|id| format!("`{id}`"))
        .collect::<Vec<_>>()
        .join(", ");
    write!(f, "; did you mean {quoted}?")
}

/// All problems found in one theme file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeFileError {
    /// File path for user themes; `None` for embedded built-ins / in-memory text.
    pub path: Option<PathBuf>,
    /// File stem (the expected id).
    pub stem: String,
    pub errors: Vec<ThemeError>,
}

impl fmt::Display for ThemeFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "theme file {}", path.display())?,
            None => write!(f, "theme `{}`", self.stem)?,
        }
        write!(f, " is invalid:")?;
        for error in &self.errors {
            write!(f, "\n  - {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ThemeFileError {}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTheme {
    pub id: Option<String>,
    pub name: Option<String>,
    pub aliases: Option<Vec<String>>,
    pub family: Option<String>,
    pub extends: Option<String>,
    pub palette: Option<RawPalette>,
    pub tui: Option<RawTui>,
    pub widget: Option<RawWidget>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawPalette {
    pub bg: Option<String>,
    pub fg: Option<String>,
    pub accent: Option<String>,
    pub secondary_accent: Option<String>,
    pub border_active: Option<String>,
    pub border_inactive: Option<String>,
    pub text_muted: Option<String>,
    pub success: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawTui {
    pub glyph_style: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawWidget {
    pub renderer: Option<String>,
    pub font: Option<String>,
    pub pixel_scale: Option<i64>,
    pub shades: Option<Vec<String>>,
    pub density: Option<String>,
    pub border: Option<String>,
    pub radius: Option<i64>,
    pub dither: Option<String>,
    pub glow: Option<f64>,
    pub modules: Option<Vec<String>>,
}

impl RawTheme {
    pub(super) fn parse(text: &str) -> Result<Self, ThemeError> {
        toml::from_str(text).map_err(|error| ThemeError::Parse {
            message: error.message().trim().to_owned(),
        })
    }

    /// Overlays `self` (the child) onto `base`. Aliases and `extends` are
    /// never inherited: aliases belong to exactly one theme.
    pub(super) fn merged_over(self, base: &Self) -> Self {
        Self {
            id: self.id,
            name: self.name.or_else(|| base.name.clone()),
            aliases: self.aliases,
            family: self.family.or_else(|| base.family.clone()),
            extends: None,
            palette: merge_option(self.palette, base.palette.as_ref(), RawPalette::merged_over),
            tui: merge_option(self.tui, base.tui.as_ref(), |child, base| RawTui {
                glyph_style: child.glyph_style.or_else(|| base.glyph_style.clone()),
            }),
            widget: merge_option(self.widget, base.widget.as_ref(), RawWidget::merged_over),
        }
    }
}

fn merge_option<T: Clone>(
    child: Option<T>,
    base: Option<&T>,
    merge: impl Fn(T, &T) -> T,
) -> Option<T> {
    match (child, base) {
        (Some(child), Some(base)) => Some(merge(child, base)),
        (Some(child), None) => Some(child),
        (None, base) => base.cloned(),
    }
}

impl RawPalette {
    fn merged_over(self, base: &Self) -> Self {
        Self {
            bg: self.bg.or_else(|| base.bg.clone()),
            fg: self.fg.or_else(|| base.fg.clone()),
            accent: self.accent.or_else(|| base.accent.clone()),
            secondary_accent: self
                .secondary_accent
                .or_else(|| base.secondary_accent.clone()),
            border_active: self.border_active.or_else(|| base.border_active.clone()),
            border_inactive: self
                .border_inactive
                .or_else(|| base.border_inactive.clone()),
            text_muted: self.text_muted.or_else(|| base.text_muted.clone()),
            success: self.success.or_else(|| base.success.clone()),
            warning: self.warning.or_else(|| base.warning.clone()),
            error: self.error.or_else(|| base.error.clone()),
        }
    }

    fn get(&self, key: &str) -> Option<&String> {
        match key {
            "bg" => self.bg.as_ref(),
            "fg" => self.fg.as_ref(),
            "accent" => self.accent.as_ref(),
            "secondary_accent" => self.secondary_accent.as_ref(),
            "border_active" => self.border_active.as_ref(),
            "border_inactive" => self.border_inactive.as_ref(),
            "text_muted" => self.text_muted.as_ref(),
            "success" => self.success.as_ref(),
            "warning" => self.warning.as_ref(),
            "error" => self.error.as_ref(),
            _ => None,
        }
    }
}

impl RawWidget {
    fn merged_over(self, base: &Self) -> Self {
        Self {
            renderer: self.renderer.or_else(|| base.renderer.clone()),
            font: self.font.or_else(|| base.font.clone()),
            pixel_scale: self.pixel_scale.or(base.pixel_scale),
            shades: self.shades.or_else(|| base.shades.clone()),
            density: self.density.or_else(|| base.density.clone()),
            border: self.border.or_else(|| base.border.clone()),
            radius: self.radius.or(base.radius),
            dither: self.dither.or_else(|| base.dither.clone()),
            glow: self.glow.or(base.glow),
            modules: self.modules.or_else(|| base.modules.clone()),
        }
    }
}

/// `[a-z0-9_]+`, bounded length.
#[must_use]
pub(super) fn is_valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// Validates a fully merged raw theme and applies family defaults.
///
/// `file_stem` is checked against `id` when given. All problems are
/// collected rather than stopping at the first one.
pub(super) fn resolve(
    raw: &RawTheme,
    file_stem: Option<&str>,
    source: ThemeSource,
    extends: Option<String>,
) -> Result<Theme, Vec<ThemeError>> {
    let mut errors = Vec::new();

    let id = match raw.id.as_deref() {
        None => {
            errors.push(ThemeError::MissingField { field: "id" });
            None
        }
        Some(id) if !is_valid_id(id) => {
            errors.push(ThemeError::InvalidId {
                field: String::from("id"),
                value: id.to_owned(),
            });
            None
        }
        Some(id) => Some(id.to_owned()),
    };
    if let (Some(id), Some(stem)) = (id.as_deref(), file_stem) {
        if id != stem {
            errors.push(ThemeError::IdMismatch {
                id: id.to_owned(),
                file_stem: stem.to_owned(),
            });
        }
    }

    let name = match raw.name.as_deref().map(str::trim) {
        None => {
            errors.push(ThemeError::MissingField { field: "name" });
            None
        }
        Some(name)
            if name.is_empty()
                || name.chars().count() > MAX_NAME_CHARS
                || name.chars().any(char::is_control) =>
        {
            errors.push(ThemeError::InvalidValue {
                field: String::from("name"),
                value: name.to_owned(),
                expected: format!("1..={MAX_NAME_CHARS} printable characters"),
            });
            None
        }
        Some(name) => Some(name.to_owned()),
    };

    let aliases = resolve_aliases(
        raw.aliases.as_deref().unwrap_or_default(),
        id.as_deref(),
        &mut errors,
    );

    let family = match raw.family.as_deref() {
        None => {
            errors.push(ThemeError::MissingField { field: "family" });
            None
        }
        Some(value) => Family::parse(value).or_else(|| {
            errors.push(ThemeError::UnknownFamily {
                value: value.to_owned(),
            });
            None
        }),
    };

    let palette = resolve_palette(raw.palette.as_ref(), &mut errors);
    let tui = resolve_tui(raw.tui.as_ref(), &mut errors);
    let widget = family.zip(palette).and_then(|(family, palette)| {
        resolve_widget(raw.widget.as_ref(), family, &palette, &mut errors)
    });
    // Widget values are still validated when family/palette failed, so the
    // user sees every problem in one pass.
    if family.is_none() || palette.is_none() {
        resolve_widget(
            raw.widget.as_ref(),
            Family::Terminal,
            &placeholder_palette(),
            &mut errors,
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }
    match (id, name, family, palette, widget) {
        (Some(id), Some(name), Some(family), Some(palette), Some(widget)) => Ok(Theme {
            id,
            name,
            aliases,
            family,
            palette,
            tui,
            widget,
            source,
            extends,
        }),
        _ => Err(vec![ThemeError::Parse {
            message: String::from("theme could not be resolved"),
        }]),
    }
}

fn placeholder_palette() -> Palette {
    let black = Rgb::new(0, 0, 0);
    Palette {
        bg: black,
        fg: black,
        accent: black,
        secondary_accent: black,
        border_active: black,
        border_inactive: black,
        text_muted: black,
        success: black,
        warning: black,
        error: black,
    }
}

fn resolve_aliases(raw: &[String], id: Option<&str>, errors: &mut Vec<ThemeError>) -> Vec<String> {
    if raw.len() > MAX_ALIASES {
        errors.push(ThemeError::InvalidValue {
            field: String::from("aliases"),
            value: format!("{} entries", raw.len()),
            expected: format!("at most {MAX_ALIASES} aliases"),
        });
    }
    let mut aliases: Vec<String> = Vec::with_capacity(raw.len());
    for alias in raw {
        if !is_valid_id(alias) {
            errors.push(ThemeError::InvalidId {
                field: String::from("aliases"),
                value: alias.clone(),
            });
        } else if Some(alias.as_str()) == id || aliases.contains(alias) {
            errors.push(ThemeError::InvalidValue {
                field: String::from("aliases"),
                value: alias.clone(),
                expected: String::from("unique aliases that differ from the theme id"),
            });
        } else {
            aliases.push(alias.clone());
        }
    }
    aliases
}

fn parse_colour(field: String, value: &str, errors: &mut Vec<ThemeError>) -> Option<Rgb> {
    Rgb::parse_hex(value).or_else(|| {
        errors.push(ThemeError::BadHex {
            field,
            value: value.to_owned(),
        });
        None
    })
}

fn resolve_palette(raw: Option<&RawPalette>, errors: &mut Vec<ThemeError>) -> Option<Palette> {
    let empty = RawPalette::default();
    let raw = raw.unwrap_or(&empty);
    let mut colours = [Rgb::new(0, 0, 0); 10];
    let mut ok = true;
    for (slot, key) in colours.iter_mut().zip(Palette::KEYS) {
        match raw.get(key) {
            None => {
                errors.push(ThemeError::MissingPaletteKey { key });
                ok = false;
            }
            Some(value) => match parse_colour(format!("palette.{key}"), value, errors) {
                Some(colour) => *slot = colour,
                None => ok = false,
            },
        }
    }
    ok.then_some(Palette {
        bg: colours[0],
        fg: colours[1],
        accent: colours[2],
        secondary_accent: colours[3],
        border_active: colours[4],
        border_inactive: colours[5],
        text_muted: colours[6],
        success: colours[7],
        warning: colours[8],
        error: colours[9],
    })
}

fn resolve_tui(raw: Option<&RawTui>, errors: &mut Vec<ThemeError>) -> TuiStyle {
    let mut style = TuiStyle::default();
    if let Some(value) = raw.and_then(|raw| raw.glyph_style.as_deref()) {
        match GlyphStyle::parse(value) {
            Some(glyph_style) => style.glyph_style = glyph_style,
            None => errors.push(invalid("tui.glyph_style", value, GlyphStyle::expected())),
        }
    }
    style
}

fn invalid(field: &str, value: impl fmt::Display, expected: impl Into<String>) -> ThemeError {
    ThemeError::InvalidValue {
        field: field.to_owned(),
        value: value.to_string(),
        expected: expected.into(),
    }
}

fn parse_enum<T: Copy>(
    field: &str,
    raw: Option<&str>,
    default: T,
    parse: fn(&str) -> Option<T>,
    expected: fn() -> String,
    errors: &mut Vec<ThemeError>,
) -> T {
    match raw {
        None => default,
        Some(value) => parse(value).unwrap_or_else(|| {
            errors.push(invalid(field, value, expected()));
            default
        }),
    }
}

#[allow(clippy::too_many_lines)]
fn resolve_widget(
    raw: Option<&RawWidget>,
    family: Family,
    palette: &Palette,
    errors: &mut Vec<ThemeError>,
) -> Option<WidgetStyle> {
    let empty = RawWidget::default();
    let raw = raw.unwrap_or(&empty);
    let defaults = family.defaults();
    let before = errors.len();

    let renderer = parse_enum(
        "widget.renderer",
        raw.renderer.as_deref(),
        defaults.renderer,
        Renderer::parse,
        Renderer::expected,
        errors,
    );
    let font = parse_enum(
        "widget.font",
        raw.font.as_deref(),
        defaults.font,
        Font::parse,
        Font::expected,
        errors,
    );
    let density = parse_enum(
        "widget.density",
        raw.density.as_deref(),
        defaults.density,
        Density::parse,
        Density::expected,
        errors,
    );
    let border = parse_enum(
        "widget.border",
        raw.border.as_deref(),
        defaults.border,
        Border::parse,
        Border::expected,
        errors,
    );
    let dither = parse_enum(
        "widget.dither",
        raw.dither.as_deref(),
        defaults.dither,
        Dither::parse,
        Dither::expected,
        errors,
    );

    let pixel_scale = match raw.pixel_scale {
        None => 0,
        Some(value) => u8::try_from(value)
            .ok()
            .filter(|value| *value <= MAX_PIXEL_SCALE)
            .unwrap_or_else(|| {
                errors.push(invalid(
                    "widget.pixel_scale",
                    value,
                    format!("0..={MAX_PIXEL_SCALE} (0 = auto)"),
                ));
                0
            }),
    };
    let radius = match raw.radius {
        None => defaults.radius,
        Some(value) => u8::try_from(value)
            .ok()
            .filter(|value| *value <= MAX_RADIUS)
            .unwrap_or_else(|| {
                errors.push(invalid("widget.radius", value, format!("0..={MAX_RADIUS}")));
                defaults.radius
            }),
    };
    let glow = match raw.glow {
        None => defaults.glow,
        Some(value) if value.is_finite() && (0.0..=1.0).contains(&value) => value,
        Some(value) => {
            errors.push(invalid("widget.glow", value, "0.0..=1.0"));
            defaults.glow
        }
    };

    let shades = match raw.shades.as_deref() {
        None => derived_shades(palette),
        Some(values) => {
            if !(MIN_SHADES..=MAX_SHADES).contains(&values.len()) {
                errors.push(invalid(
                    "widget.shades",
                    format!("{} entries", values.len()),
                    format!("{MIN_SHADES}..={MAX_SHADES} colours, dark to light"),
                ));
            }
            values
                .iter()
                .enumerate()
                .filter_map(|(index, value)| {
                    parse_colour(format!("widget.shades[{index}]"), value, errors)
                })
                .collect()
        }
    };

    let modules = match raw.modules.as_deref() {
        None => Module::ALL.to_vec(),
        Some(values) => {
            if values.is_empty() {
                errors.push(invalid("widget.modules", "[]", "at least one module"));
            }
            let mut modules = Vec::with_capacity(values.len());
            for value in values {
                match Module::parse(value) {
                    Some(module) if modules.contains(&module) => {
                        errors.push(invalid("widget.modules", value, "each module at most once"));
                    }
                    Some(module) => modules.push(module),
                    None => errors.push(invalid("widget.modules", value, Module::expected())),
                }
            }
            modules
        }
    };

    (errors.len() == before).then_some(WidgetStyle {
        renderer,
        font,
        pixel_scale,
        shades,
        density,
        border,
        radius,
        dither,
        glow,
        modules,
    })
}

#[derive(Serialize)]
struct ExportTheme<'a> {
    id: &'a str,
    name: &'a str,
    aliases: &'a [String],
    family: Family,
    palette: &'a Palette,
    tui: &'a TuiStyle,
    widget: &'a WidgetStyle,
}

pub(super) fn render_theme(theme: &Theme) -> String {
    let export = ExportTheme {
        id: &theme.id,
        name: &theme.name,
        aliases: &theme.aliases,
        family: theme.family,
        palette: &theme.palette,
        tui: &theme.tui,
        widget: &theme.widget,
    };
    // Every field is a plain scalar/array/table: serialisation cannot fail.
    toml::to_string(&export).unwrap_or_default()
}
