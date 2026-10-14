use ratatui::style::{Color, Modifier, Style};

use crate::config::ThemeId;
pub use crate::theme::GlyphStyle;

/// Linear blend between two theme colours; non-RGB colours switch halfway.
pub(crate) fn mix(from: Color, to: Color, amount: f64) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    match (from, to) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let blend = |a: u8, b: u8| {
                (f64::from(a) + (f64::from(b) - f64::from(a)) * amount).round() as u8
            };
            Color::Rgb(blend(r1, r2), blend(g1, g2), blend(b1, b2))
        }
        _ if amount < 0.5 => from,
        _ => to,
    }
}

/// TUI accessors for the configured theme. Themes come from the shared
/// [theme registry](crate::theme); names that no longer resolve fall back to
/// the default theme so rendering never fails.
impl ThemeId {
    /// Every registry theme id, in registry (id) order.
    #[must_use]
    pub fn all() -> Vec<Self> {
        crate::theme::registry()
            .themes()
            .iter()
            .map(|theme| Self::new(theme.id.clone()))
            .collect()
    }

    #[must_use]
    pub fn name(&self) -> String {
        self.resolve_or_default().name.clone()
    }

    #[must_use]
    pub fn glyph_style(&self) -> GlyphStyle {
        self.resolve_or_default().tui.glyph_style
    }

    #[must_use]
    pub fn palette(&self) -> Palette {
        Palette::from(self.resolve_or_default().palette)
    }

    #[must_use]
    pub fn styles(&self) -> SemanticStyles {
        self.palette().styles()
    }
}

impl From<crate::theme::Palette> for Palette {
    fn from(palette: crate::theme::Palette) -> Self {
        let rgb = |colour: crate::theme::Rgb| Color::Rgb(colour.r, colour.g, colour.b);
        Self {
            bg: rgb(palette.bg),
            fg: rgb(palette.fg),
            accent: rgb(palette.accent),
            secondary_accent: rgb(palette.secondary_accent),
            border_active: rgb(palette.border_active),
            border_inactive: rgb(palette.border_inactive),
            text_muted: rgb(palette.text_muted),
            success: rgb(palette.success),
            warning: rgb(palette.warning),
            error: rgb(palette.error),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub secondary_accent: Color,
    pub border_active: Color,
    pub border_inactive: Color,
    pub text_muted: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
}

impl Palette {
    #[must_use]
    pub fn to_hex(color: Color) -> String {
        match color {
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            Color::Black => String::from("#000000"),
            Color::White => String::from("#ffffff"),
            Color::DarkGray => String::from("#555555"),
            Color::Gray => String::from("#aaaaaa"),
            _ => String::from("#ffb000"),
        }
    }

    #[must_use]
    pub fn styles(&self) -> SemanticStyles {
        SemanticStyles::from_palette(self)
    }
}

/// Semantic Ratatui styles derived purely from a [`Palette`].
///
/// This layer decouples raw theme colors (source palette) from the
/// presentation semantics used across TUI rendering surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticStyles {
    /// Underlying source palette.
    pub palette: Palette,

    // Base / Canvas
    /// Background container / canvas style.
    pub base: Style,
    /// Container background fill.
    pub container_bg: Style,

    // Typography / Text roles
    /// Standard reading text.
    pub text_primary: Style,
    /// Dimmed or secondary metadata text.
    pub text_muted: Style,
    /// Primary view or section headers.
    pub text_heading: Style,
    /// Subsection or category headers.
    pub text_section: Style,

    // Structural / Borders
    /// Standard inactive boundary or container edge.
    pub border_normal: Style,
    /// Active, focused, or highlighted container boundary.
    pub border_focused: Style,

    // Interaction states
    /// Default unselected list/field item.
    pub item_normal: Style,
    /// Item under navigation focus.
    pub item_focused: Style,
    /// Item selected or active in a list.
    pub item_selected: Style,
    /// Item simultaneously focused and selected.
    pub item_focused_selected: Style,
    /// Field currently in active text edit mode.
    pub item_editing: Style,
    /// Value modified but unsaved.
    pub item_modified: Style,
    /// Disabled or non-interactive item.
    pub item_disabled: Style,

    // Operational Status & Alerts
    /// Normal, healthy, or confirmed operation.
    pub status_success: Style,
    /// In-progress, transitional, or cautionary operation.
    pub status_warning: Style,
    /// Failed, offline, or rejected operation.
    pub status_error: Style,

    // Navigation & Chrome
    /// Application brand and status header title.
    pub chrome_title: Style,
    /// Secondary tone for the application brand header/masthead stripe.
    pub chrome_title_secondary: Style,
    /// Currently active tab strip item.
    pub tab_active: Style,
    /// Inactive tab strip item.
    pub tab_inactive: Style,
    /// Shortcut key label in footer or help overlays.
    pub key_hint: Style,
    /// Description label accompanying a key hint.
    pub key_hint_desc: Style,

    // Badges
    /// Status badge for an active/live daemon connection.
    pub badge_live: Style,
    /// Status badge for an offline/unreachable daemon.
    pub badge_offline: Style,
    /// Status badge for cautionary daemon states (suspended, idle).
    pub badge_warning: Style,

    // Command Bar Modes
    /// Mode indicator for normal navigation.
    pub mode_nav: Style,
    /// Mode indicator for text/field editing.
    pub mode_edit: Style,
    /// Mode indicator for milestone adjustment.
    pub mode_adjust: Style,

    // Instrument & Master/Detail Roles
    /// Selection/focus indicator marker (e.g. ▌).
    pub focus_marker: Style,
    /// Timeline/progress indicator marker (e.g. •) for live schedule position.
    pub current_marker: Style,
    /// Active fill portion of an instrument gauge.
    pub gauge_fill: Style,
    /// Inactive track portion of an instrument gauge.
    pub gauge_track: Style,
    /// Label in an aligned data/summary table.
    pub data_label: Style,
    /// Prominent value in an aligned data/summary table.
    pub data_value: Style,

    // Focused / Editing Value Capsules
    /// Value capsule treatment for the currently focused interactive setting.
    pub value_capsule_focused: Style,
    /// Value capsule treatment for the setting currently in editing mode.
    pub value_capsule_editing: Style,
}

/// Helper to compute perceived perceptual luminance of a color (0.0 to 255.0).
#[must_use]
pub fn color_luminance(c: Color) -> f64 {
    match c {
        Color::Rgb(r, g, b) => 0.299 * f64::from(r) + 0.587 * f64::from(g) + 0.114 * f64::from(b),
        Color::Black => 0.0,
        Color::White => 255.0,
        _ => 128.0,
    }
}

/// Computes a contrast-safe text foreground color derived directly from the theme's palette,
/// ensuring readability on accent capsules while preserving theme-coherent color identity.
#[must_use]
pub fn contrast_fg_for_palette(bg: Color, palette: &Palette) -> Color {
    let bg_lum = color_luminance(bg);
    let lum_background = color_luminance(palette.bg);
    let lum_foreground = color_luminance(palette.fg);

    let bg_diff = (bg_lum - lum_background).abs();
    let fg_diff = (bg_lum - lum_foreground).abs();

    if bg_diff >= fg_diff && bg_diff > 40.0 {
        palette.bg
    } else if fg_diff > 40.0 {
        palette.fg
    } else if bg_lum > 128.0 {
        Color::Black
    } else {
        Color::White
    }
}

impl SemanticStyles {
    /// Pure derivation of semantic styles from a source palette.
    #[must_use]
    pub fn from_palette(palette: &Palette) -> Self {
        let capsule_fg = contrast_fg_for_palette(palette.accent, palette);
        let edit_capsule_fg = contrast_fg_for_palette(palette.secondary_accent, palette);

        Self {
            palette: *palette,
            base: Style::default().bg(palette.bg).fg(palette.fg),
            container_bg: Style::default().bg(palette.bg),
            text_primary: Style::default().fg(palette.fg),
            text_muted: Style::default().fg(palette.text_muted),
            text_heading: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            text_section: Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
            border_normal: Style::default().fg(palette.border_inactive),
            border_focused: Style::default().fg(palette.border_active),
            item_normal: Style::default().fg(palette.fg),
            item_focused: Style::default().fg(palette.accent),
            item_selected: Style::default().bg(palette.accent).fg(palette.bg),
            item_focused_selected: Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(Modifier::BOLD),
            item_editing: Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            item_modified: Style::default().fg(palette.warning),
            item_disabled: Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM),
            status_success: Style::default().fg(palette.success),
            status_warning: Style::default().fg(palette.warning),
            status_error: Style::default().fg(palette.error),
            chrome_title: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            chrome_title_secondary: Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
            tab_active: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            tab_inactive: Style::default().fg(palette.text_muted),
            key_hint: Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
            key_hint_desc: Style::default().fg(palette.text_muted),
            badge_live: Style::default()
                .fg(palette.bg)
                .bg(palette.success)
                .add_modifier(Modifier::BOLD),
            badge_offline: Style::default()
                .fg(palette.bg)
                .bg(palette.error)
                .add_modifier(Modifier::BOLD),
            badge_warning: Style::default()
                .fg(palette.bg)
                .bg(palette.warning)
                .add_modifier(Modifier::BOLD),
            mode_nav: Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
            mode_edit: Style::default()
                .fg(palette.bg)
                .bg(palette.warning)
                .add_modifier(Modifier::BOLD),
            mode_adjust: Style::default()
                .fg(palette.bg)
                .bg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
            focus_marker: Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD),
            current_marker: Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD),
            gauge_fill: Style::default().fg(palette.accent),
            gauge_track: Style::default().fg(palette.border_inactive),
            data_label: Style::default().fg(palette.text_muted),
            data_value: Style::default().fg(palette.fg).add_modifier(Modifier::BOLD),
            value_capsule_focused: Style::default()
                .bg(palette.accent)
                .fg(capsule_fg)
                .add_modifier(Modifier::BOLD),
            value_capsule_editing: Style::default()
                .bg(palette.secondary_accent)
                .fg(edit_capsule_fg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        }
    }

    /// Derives the style for typed daemon lifecycle state.
    #[must_use]
    pub fn daemon_lifecycle_status(&self, lifecycle: crate::tui::model::DaemonLifecycle) -> Style {
        match lifecycle {
            crate::tui::model::DaemonLifecycle::Active => self.status_success,
            crate::tui::model::DaemonLifecycle::IdleDimmed => {
                self.status_warning.add_modifier(Modifier::BOLD)
            }
            crate::tui::model::DaemonLifecycle::Suspended => {
                self.status_warning.add_modifier(Modifier::BOLD)
            }
            crate::tui::model::DaemonLifecycle::Unreachable => self.status_error,
        }
    }
}

impl From<&Palette> for SemanticStyles {
    fn from(palette: &Palette) -> Self {
        Self::from_palette(palette)
    }
}

impl From<Palette> for SemanticStyles {
    fn from(palette: Palette) -> Self {
        Self::from_palette(&palette)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_blend_only_when_both_are_rgb() {
        assert_eq!(
            mix(Color::Rgb(0, 0, 0), Color::Rgb(200, 100, 50), 0.5),
            Color::Rgb(100, 50, 25)
        );
        assert_eq!(mix(Color::Blue, Color::Rgb(1, 1, 1), 0.2), Color::Blue);
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn test_semantic_styles_pure_derivation_from_palette() {
        let palette = ThemeId::new("amber").palette();
        let styles = palette.styles();

        assert_eq!(styles.palette, palette);
        assert_eq!(styles.base, Style::default().bg(palette.bg).fg(palette.fg));
        assert_eq!(styles.container_bg, Style::default().bg(palette.bg));
        assert_eq!(styles.text_primary, Style::default().fg(palette.fg));
        assert_eq!(styles.text_muted, Style::default().fg(palette.text_muted));
        assert_eq!(
            styles.text_heading,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.text_section,
            Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.border_normal,
            Style::default().fg(palette.border_inactive)
        );
        assert_eq!(
            styles.border_focused,
            Style::default().fg(palette.border_active)
        );
        assert_eq!(styles.item_normal, Style::default().fg(palette.fg));
        assert_eq!(styles.item_focused, Style::default().fg(palette.accent));
        assert_eq!(
            styles.item_selected,
            Style::default().bg(palette.accent).fg(palette.bg)
        );
        assert_eq!(
            styles.item_focused_selected,
            Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.item_editing,
            Style::default()
                .bg(palette.accent)
                .fg(palette.bg)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
        );
        assert_eq!(styles.item_modified, Style::default().fg(palette.warning));
        assert_eq!(
            styles.item_disabled,
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM)
        );
        assert_eq!(styles.status_success, Style::default().fg(palette.success));
        assert_eq!(styles.status_warning, Style::default().fg(palette.warning));
        assert_eq!(styles.status_error, Style::default().fg(palette.error));
        assert_eq!(
            styles.chrome_title,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.tab_active,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(styles.tab_inactive, Style::default().fg(palette.text_muted));
        assert_eq!(
            styles.key_hint,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.key_hint_desc,
            Style::default().fg(palette.text_muted)
        );
        assert_eq!(
            styles.badge_live,
            Style::default()
                .fg(palette.bg)
                .bg(palette.success)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.badge_offline,
            Style::default()
                .fg(palette.bg)
                .bg(palette.error)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.badge_warning,
            Style::default()
                .fg(palette.bg)
                .bg(palette.warning)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.mode_nav,
            Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.mode_edit,
            Style::default()
                .fg(palette.bg)
                .bg(palette.warning)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.mode_adjust,
            Style::default()
                .fg(palette.bg)
                .bg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.focus_marker,
            Style::default()
                .fg(palette.accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.current_marker,
            Style::default()
                .fg(palette.secondary_accent)
                .add_modifier(Modifier::BOLD)
        );
        assert_eq!(styles.gauge_fill, Style::default().fg(palette.accent));
        assert_eq!(
            styles.gauge_track,
            Style::default().fg(palette.border_inactive)
        );
        assert_eq!(styles.data_label, Style::default().fg(palette.text_muted));
        assert_eq!(
            styles.data_value,
            Style::default().fg(palette.fg).add_modifier(Modifier::BOLD)
        );
    }

    #[test]
    fn test_interaction_states_are_distinguishable() {
        let styles = ThemeId::new("amber").styles();

        assert_ne!(styles.item_normal, styles.item_focused);
        assert_ne!(styles.item_focused, styles.item_selected);
        assert_ne!(styles.item_selected, styles.item_focused_selected);
        assert_ne!(styles.item_focused_selected, styles.item_editing);
        assert_ne!(styles.item_editing, styles.item_disabled);
        assert_ne!(styles.item_normal, styles.item_disabled);
        assert_ne!(styles.status_success, styles.status_warning);
        assert_ne!(styles.status_warning, styles.status_error);
        assert_ne!(styles.border_normal, styles.border_focused);
    }

    #[test]
    fn test_retro_monochrome_themes_intentionally_reuse_colors() {
        // Commodore 64 uses white for both success and warning
        let c64 = ThemeId::new("commodore64").palette();
        assert_eq!(c64.success, c64.warning);

        // Grayscale uses white for accent, border_active, and success
        let gray = ThemeId::new("grayscale").palette();
        assert_eq!(gray.accent, gray.border_active);
        assert_eq!(gray.accent, gray.success);

        // HackerGreen uses pure green for fg, border_active, and success
        let green = ThemeId::new("hackergreen").palette();
        assert_eq!(green.fg, green.border_active);
        assert_eq!(green.fg, green.success);

        // Every theme resolves valid semantic styles without panicking
        for theme in ThemeId::all() {
            let styles = theme.styles();
            assert_ne!(styles.palette.bg, styles.palette.fg);
        }
    }

    #[test]
    fn test_daemon_status_derivation() {
        use crate::tui::model::DaemonLifecycle;

        let styles = ThemeId::new("amber").styles();

        assert_eq!(
            styles.daemon_lifecycle_status(DaemonLifecycle::Active),
            styles.status_success
        );
        assert_eq!(
            styles.daemon_lifecycle_status(DaemonLifecycle::IdleDimmed),
            styles.status_warning.add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.daemon_lifecycle_status(DaemonLifecycle::Suspended),
            styles.status_warning.add_modifier(Modifier::BOLD)
        );
        assert_eq!(
            styles.daemon_lifecycle_status(DaemonLifecycle::Unreachable),
            styles.status_error
        );
    }

    #[test]
    fn test_from_conversions_match() {
        let palette = ThemeId::new("dracula").palette();
        let from_ref = SemanticStyles::from(&palette);
        let from_val = SemanticStyles::from(palette);
        let from_theme = ThemeId::new("dracula").styles();

        assert_eq!(from_ref, from_val);
        assert_eq!(from_ref, from_theme);
    }

    #[test]
    fn test_all_themes_resolve_palette_and_deserialize() {
        // Legacy enum serde names (the old `config::Theme` variants).
        let legacy_names = [
            "amber",
            "amiga",
            "ayudark",
            "ayumirage",
            "braun",
            "casiodigital",
            "catppuccinmocha",
            "classicmacintosh",
            "commodore64",
            "cyberpunk",
            "dracula",
            "everforest",
            "grayscale",
            "gruvbox",
            "hackergreen",
            "handheld",
            "kanagawa",
            "materialocean",
            "monokai",
            "nightowl",
            "nixie",
            "nord",
            "nothing",
            "onedark",
            "oscilloscope",
            "phosphorblue",
            "rosepine",
            "solarizeddark",
            "synthwave84",
            "terminal",
            "thinkpad",
            "tokyonight",
            "unixworkstation",
            "vfdhifi",
            "zenburn",
        ];

        assert_eq!(ThemeId::all().len(), 40);
        assert_eq!(legacy_names.len(), 35);

        let added = ["ps3", "winxp", "macos", "wii", "nds"];
        let legacy_ids = ThemeId::all()
            .into_iter()
            .filter(|id| !added.contains(&id.as_str()));
        for (key, expected) in legacy_names.into_iter().zip(legacy_ids) {
            let toml_str = format!("theme = \"{key}\"\n");
            let parsed: Result<crate::config::TuiConfig, _> = toml::from_str(&toml_str);
            assert!(
                parsed.is_ok(),
                "Failed to deserialize theme '{key}': {:?}",
                parsed.err()
            );
            assert_eq!(parsed.unwrap().theme, expected);

            let palette = expected.palette();
            assert_ne!(palette.bg, palette.fg);
            assert!(!expected.name().is_empty());
        }
    }

    #[test]
    fn test_theme_statoscope_aliases_deserialize() {
        let aliases = [
            ("din", "braun"),
            ("workbench", "amiga"),
            ("vfd", "vfd_hifi"),
            ("vfd_hifi", "vfd_hifi"),
            ("hifi", "vfd_hifi"),
            ("gameboy", "handheld"),
            ("dmg", "handheld"),
            ("dmg01", "handheld"),
            ("unix", "unixworkstation"),
            ("workstation", "unixworkstation"),
            ("motif", "unixworkstation"),
            ("irix", "unixworkstation"),
            ("tubes", "nixie"),
            ("nixie_tubes", "nixie"),
            ("scope", "oscilloscope"),
            ("crt", "oscilloscope"),
            ("glyph", "nothing"),
            ("casio", "casiodigital"),
            ("digitalwatch", "casiodigital"),
            ("digital_watch", "casiodigital"),
            ("macintosh128k", "classicmacintosh"),
            ("mac128k", "classicmacintosh"),
            ("c64", "commodore64"),
        ];

        for (alias, expected) in aliases {
            let toml_str = format!("theme = \"{alias}\"\n");
            let parsed: Result<crate::config::TuiConfig, _> = toml::from_str(&toml_str);
            assert!(
                parsed.is_ok(),
                "Failed to deserialize alias '{alias}': {:?}",
                parsed.err()
            );
            assert_eq!(
                parsed.unwrap().theme,
                ThemeId::new(expected),
                "Alias '{alias}' did not resolve to expected theme"
            );
        }
    }

    #[test]
    fn test_unknown_theme_is_kept_for_validation_and_falls_back_when_rendering() {
        let parsed: crate::config::TuiConfig = toml::from_str("theme = \"no_such\"\n").unwrap();
        assert_eq!(parsed.theme, ThemeId::new("no_such"));
        assert_eq!(parsed.theme.palette(), ThemeId::default().palette());
        let config = crate::config::Config {
            tui: parsed,
            ..crate::config::Config::default()
        };
        // Cosmetic: the core validation (daemon load path) accepts it ...
        config
            .validate()
            .expect("theme problems never fail core validation");
        // ... while the strict check used by `config validate` / doctor rejects it.
        let error = config.validate_theme().unwrap_err().to_string();
        assert!(
            error.contains("tui.theme: unknown theme `no_such`"),
            "{error}"
        );
    }

    #[test]
    fn test_theme_serialization_stays_a_plain_string() {
        let config = crate::config::TuiConfig {
            theme: ThemeId::new("handheld"),
            ..crate::config::TuiConfig::default()
        };
        let rendered = toml::to_string(&config).unwrap();
        assert!(rendered.contains("theme = \"handheld\""), "{rendered}");
        let legacy: crate::config::TuiConfig = toml::from_str("theme = \"workbench\"").unwrap();
        assert!(toml::to_string(&legacy)
            .unwrap()
            .contains("theme = \"amiga\""));
        assert_eq!(crate::config::TuiConfig::default().theme.as_str(), "amber");
    }

    #[test]
    fn test_phase9_3_contrast_safety_across_representative_themes() {
        for theme in [
            ThemeId::new("amber"),
            ThemeId::new("terminal"),
            ThemeId::new("nord"),
            ThemeId::new("tokyonight"),
            ThemeId::new("hackergreen"),
            ThemeId::new("grayscale"),
            ThemeId::new("commodore64"),
            ThemeId::new("synthwave84"),
        ] {
            let palette = theme.palette();
            let styles = palette.styles();

            // 1. Dual-tone masthead colors are distinct and valid
            assert_ne!(styles.chrome_title, styles.chrome_title_secondary);

            // 2. Focused value capsule uses palette accent and has contrasting text
            assert_eq!(styles.value_capsule_focused.bg, Some(palette.accent));
            assert_eq!(
                styles.value_capsule_editing.bg,
                Some(palette.secondary_accent)
            );
            assert_ne!(styles.value_capsule_editing.bg, Some(palette.warning));

            // 3. Capsule fg is derived from palette bg or fg or black/white
            let fg = styles.value_capsule_focused.fg.unwrap();
            assert!(
                fg == palette.bg || fg == palette.fg || fg == Color::Black || fg == Color::White
            );
        }
    }
}
