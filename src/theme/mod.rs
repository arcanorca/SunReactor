//! Theme definitions shared by the TUI, the daemon (IPC `status.theme`,
//! `list_themes`, `get_theme`) and the Plasma widget.
//!
//! Themes are data: built-ins are embedded TOML files (`src/theme/builtin`),
//! user themes live in `~/.config/sunreactor/themes/<id>.toml` and shadow a
//! built-in with the same id. See `plasma/org.sunreactor.plasma/DESIGN.md` §2.
//!
//! This module is UI-toolkit agnostic (no ratatui types) so the daemon can
//! use it without the `tui` feature.

mod builtin;
mod file;
mod registry;

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, PoisonError, RwLock};

use serde::{Serialize, Serializer};

pub use file::{ThemeError, ThemeFileError};
pub use registry::{read_theme_file, validate_file, Registry, UnknownTheme, UserThemeSource};

/// Theme id used when nothing else is configured.
pub const DEFAULT_THEME_ID: &str = "amber";

/// An sRGB colour, written as `#rrggbb` (lowercase) in files and IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses `#rrggbb` (hex digits in either case). Anything else is rejected.
    #[must_use]
    pub fn parse_hex(value: &str) -> Option<Self> {
        let digits = value.strip_prefix('#')?;
        if digits.len() != 6 || !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return None;
        }
        let channel = |range: std::ops::Range<usize>| u8::from_str_radix(&digits[range], 16).ok();
        Some(Self::new(channel(0..2)?, channel(2..4)?, channel(4..6)?))
    }

    #[must_use]
    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Perceived luminance (0..=255), Rec. 601 weights.
    #[must_use]
    pub fn luminance(self) -> f64 {
        0.299 * f64::from(self.r) + 0.587 * f64::from(self.g) + 0.114 * f64::from(self.b)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex())
    }
}

/// The ten semantic colours every theme defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Palette {
    pub bg: Rgb,
    pub fg: Rgb,
    pub accent: Rgb,
    pub secondary_accent: Rgb,
    pub border_active: Rgb,
    pub border_inactive: Rgb,
    pub text_muted: Rgb,
    pub success: Rgb,
    pub warning: Rgb,
    pub error: Rgb,
}

impl Palette {
    /// Palette keys in file order.
    pub const KEYS: [&'static str; 10] = [
        "bg",
        "fg",
        "accent",
        "secondary_accent",
        "border_active",
        "border_inactive",
        "text_muted",
        "success",
        "warning",
        "error",
    ];

    /// `(key, colour)` pairs in [`Self::KEYS`] order.
    #[must_use]
    pub fn entries(&self) -> [(&'static str, Rgb); 10] {
        [
            ("bg", self.bg),
            ("fg", self.fg),
            ("accent", self.accent),
            ("secondary_accent", self.secondary_accent),
            ("border_active", self.border_active),
            ("border_inactive", self.border_inactive),
            ("text_muted", self.text_muted),
            ("success", self.success),
            ("warning", self.warning),
            ("error", self.error),
        ]
    }
}

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        $name:ident { $($variant:ident => $text:literal),+ $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            #[must_use]
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }

            #[must_use]
            pub fn parse(value: &str) -> Option<Self> {
                match value {
                    $($text => Some(Self::$variant),)+
                    _ => None,
                }
            }

            /// Accepted spellings, for error messages.
            #[must_use]
            pub fn expected() -> String {
                [$($text),+].join(" | ")
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.serialize_str(self.as_str())
            }
        }
    };
}

string_enum! {
    /// Visual family; decides widget defaults.
    Family { Pixel => "pixel", Instrument => "instrument", Terminal => "terminal" }
}

string_enum! {
    /// TUI glyph set for curve/gauge art.
    GlyphStyle { Rounded => "rounded", Pixel => "pixel" }
}

string_enum! {
    Renderer { Pixel => "pixel", Vector => "vector", Terminal => "terminal" }
}

string_enum! {
    Font {
        Bitmap8 => "bitmap8",
        BitmapVga => "bitmap_vga",
        Mono => "mono",
        Sans => "sans",
        Display => "display",
    }
}

string_enum! {
    Density { Compact => "compact", Comfortable => "comfortable" }
}

string_enum! {
    Border { None => "none", Line => "line", Bevel => "bevel", Inset => "inset", Double => "double" }
}

string_enum! {
    Dither { None => "none", Bayer2 => "bayer2", Bayer4 => "bayer4" }
}

string_enum! {
    /// Widget popup layout modules, in default order.
    Module {
        Telemetry => "telemetry",
        Curve => "curve",
        Displays => "displays",
        Automation => "automation",
        Weather => "weather",
        Forecast => "forecast",
        Suntimes => "suntimes",
    }
}

/// TUI-specific presentation tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TuiStyle {
    pub glyph_style: GlyphStyle,
}

impl Default for TuiStyle {
    fn default() -> Self {
        Self {
            glyph_style: GlyphStyle::Rounded,
        }
    }
}

/// Widget presentation tokens with family defaults already applied.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WidgetStyle {
    pub renderer: Renderer,
    pub font: Font,
    pub pixel_scale: u8,
    pub shades: Vec<Rgb>,
    pub density: Density,
    pub border: Border,
    pub radius: u8,
    pub dither: Dither,
    pub glow: f64,
    pub modules: Vec<Module>,
}

/// Limits enforced by validation.
pub const MAX_PIXEL_SCALE: u8 = 6;
pub const MAX_RADIUS: u8 = 24;
pub const MIN_SHADES: usize = 2;
pub const MAX_SHADES: usize = 16;

/// Per-family widget defaults (DESIGN.md §2 table).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FamilyDefaults {
    pub renderer: Renderer,
    pub font: Font,
    pub border: Border,
    pub radius: u8,
    pub dither: Dither,
    pub glow: f64,
    pub density: Density,
}

impl Family {
    #[must_use]
    pub fn defaults(self) -> FamilyDefaults {
        match self {
            Self::Pixel => FamilyDefaults {
                renderer: Renderer::Pixel,
                font: Font::Bitmap8,
                border: Border::Line,
                radius: 0,
                dither: Dither::Bayer4,
                glow: 0.0,
                density: Density::Compact,
            },
            Self::Instrument => FamilyDefaults {
                renderer: Renderer::Vector,
                font: Font::Sans,
                border: Border::Line,
                radius: 4,
                dither: Dither::None,
                glow: 0.0,
                density: Density::Comfortable,
            },
            Self::Terminal => FamilyDefaults {
                renderer: Renderer::Terminal,
                font: Font::Mono,
                border: Border::Line,
                radius: 0,
                dither: Dither::None,
                glow: 0.0,
                density: Density::Compact,
            },
        }
    }
}

/// Four-step dark→light ramp derived from a palette when a theme sets no
/// explicit `widget.shades`: `bg`, `border_inactive`, `text_muted`, `fg`
/// ordered by luminance.
#[must_use]
pub fn derived_shades(palette: &Palette) -> Vec<Rgb> {
    let mut shades = vec![
        palette.bg,
        palette.border_inactive,
        palette.text_muted,
        palette.fg,
    ];
    shades.sort_by(|a, b| a.luminance().total_cmp(&b.luminance()));
    shades
}

/// Where a resolved theme came from.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ThemeSource {
    #[default]
    Builtin,
    User(PathBuf),
}

impl ThemeSource {
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User(_) => "user",
        }
    }
}

/// A fully resolved theme: every token present, family defaults applied,
/// `extends` already merged.
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub family: Family,
    pub palette: Palette,
    pub tui: TuiStyle,
    pub widget: WidgetStyle,
    pub source: ThemeSource,
    /// The id this theme extended, if any (informational).
    pub extends: Option<String>,
}

impl Theme {
    /// Renders this theme as a complete, standalone theme file (no
    /// `extends`, every key explicit). The output re-parses to an equal theme.
    #[must_use]
    pub fn to_toml(&self) -> String {
        file::render_theme(self)
    }
}

static GLOBAL_REGISTRY: RwLock<Option<Arc<Registry>>> = RwLock::new(None);

/// The process-wide registry (built-ins + user dir), loaded lazily once.
///
/// Lookups are in-memory; call [`reload_registry`] to pick up changed user
/// theme files (the daemon does this on `reload_config`).
#[must_use]
pub fn registry() -> Arc<Registry> {
    if let Some(registry) = GLOBAL_REGISTRY
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
    {
        return Arc::clone(registry);
    }
    reload_registry()
}

/// Re-reads the user theme directory and replaces the process-wide registry.
pub fn reload_registry() -> Arc<Registry> {
    let registry = Arc::new(load_default_registry());
    *GLOBAL_REGISTRY
        .write()
        .unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(&registry));
    registry
}

fn load_default_registry() -> Registry {
    // Unit tests never read the developer's real config directory.
    #[cfg(test)]
    {
        Registry::builtin()
    }
    #[cfg(not(test))]
    {
        match crate::paths::themes_dir() {
            Ok(dir) => Registry::load_dir(&dir),
            Err(error) => {
                tracing::warn!(error = %error, "theme_user_dir_unavailable");
                Registry::builtin()
            }
        }
    }
}
