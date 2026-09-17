use ratatui::style::{Color, Modifier, Style};

pub use crate::config::Theme;

impl Theme {
    pub const ALL: [Self; 28] = [
        Self::Amber,
        Self::AyuDark,
        Self::AyuMirage,
        Self::CasioDigital,
        Self::CatppuccinMocha,
        Self::ClassicMacintosh,
        Self::Commodore64,
        Self::Cyberpunk,
        Self::Dracula,
        Self::Everforest,
        Self::Grayscale,
        Self::Gruvbox,
        Self::HackerGreen,
        Self::Kanagawa,
        Self::MaterialOcean,
        Self::Monokai,
        Self::NightOwl,
        Self::Nord,
        Self::Nothing,
        Self::OneDark,
        Self::PhosphorBlue,
        Self::RosePine,
        Self::SolarizedDark,
        Self::Synthwave84,
        Self::Terminal,
        Self::ThinkPad,
        Self::TokyoNight,
        Self::Zenburn,
    ];

    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Amber => "Amber",
            Self::Terminal => "Terminal",
            Self::Dracula => "Dracula",
            Self::Gruvbox => "Gruvbox",
            Self::RosePine => "Rosé Pine",
            Self::CatppuccinMocha => "Catppuccin Mocha",
            Self::Nord => "Nord",
            Self::TokyoNight => "Tokyo Night",
            Self::OneDark => "One Dark",
            Self::AyuDark => "Ayu Dark",
            Self::AyuMirage => "Ayu Mirage",
            Self::SolarizedDark => "Solarized Dark",
            Self::Everforest => "Everforest",
            Self::Kanagawa => "Kanagawa",
            Self::Zenburn => "Zenburn",
            Self::Monokai => "Monokai",
            Self::NightOwl => "Night Owl",
            Self::MaterialOcean => "Material Ocean",
            Self::Cyberpunk => "Cyberpunk",
            Self::Synthwave84 => "Synthwave '84",
            Self::HackerGreen => "Hacker Green",
            Self::PhosphorBlue => "Phosphor Blue",
            Self::Commodore64 => "Commodore 64",
            Self::Grayscale => "Grayscale",
            Self::ClassicMacintosh => "Classic Macintosh",
            Self::CasioDigital => "Casio Digital",
            Self::Nothing => "Nothing",
            Self::ThinkPad => "ThinkPad",
        }
    }

    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn palette(self) -> Palette {
        match self {
            Self::Amber => Palette {
                bg: Color::Rgb(15, 10, 5),                 // Deep warm black
                fg: Color::Rgb(255, 210, 150),             // Bright amber-white for primary text
                accent: Color::Rgb(255, 176, 0),           // Pure amber for highlights
                secondary_accent: Color::Rgb(220, 140, 0), // Deep amber
                border_active: Color::Rgb(255, 176, 0),
                border_inactive: Color::Rgb(120, 80, 20),
                text_muted: Color::Rgb(180, 120, 50), // Muted but visible amber
                success: Color::Rgb(170, 220, 100),
                warning: Color::Rgb(255, 176, 0),
                error: Color::Rgb(255, 60, 60),
            },
            Self::Terminal => Palette {
                bg: Color::Rgb(26, 26, 26),        // #1a1a1a
                fg: Color::Rgb(80, 200, 120),      // Emerald primary text (#50c878)
                accent: Color::Rgb(140, 255, 170), // Bright green for highlights
                secondary_accent: Color::Rgb(60, 180, 100),
                border_active: Color::Rgb(80, 200, 120),
                border_inactive: Color::Rgb(40, 100, 60), // Dim green, readable against bg
                text_muted: Color::Rgb(60, 150, 90),      // Muted green, clear contrast from bg
                success: Color::Rgb(80, 200, 120),
                warning: Color::Rgb(200, 200, 0),
                error: Color::Rgb(255, 80, 80),
            },
            Self::Dracula => Palette {
                bg: Color::Rgb(40, 42, 54),
                fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(255, 121, 198),
                secondary_accent: Color::Rgb(189, 147, 249),
                border_active: Color::Rgb(255, 121, 198),
                border_inactive: Color::Rgb(98, 114, 164),
                text_muted: Color::Rgb(191, 191, 191),
                success: Color::Rgb(80, 250, 123),
                warning: Color::Rgb(241, 250, 140),
                error: Color::Rgb(255, 85, 85),
            },
            Self::Gruvbox => Palette {
                bg: Color::Rgb(40, 40, 40),
                fg: Color::Rgb(235, 219, 178),
                accent: Color::Rgb(215, 153, 33),
                secondary_accent: Color::Rgb(204, 36, 29),
                border_active: Color::Rgb(215, 153, 33),
                border_inactive: Color::Rgb(102, 92, 84),
                text_muted: Color::Rgb(168, 153, 132),
                success: Color::Rgb(152, 151, 26),
                warning: Color::Rgb(215, 153, 33),
                error: Color::Rgb(204, 36, 29),
            },
            Self::RosePine => Palette {
                bg: Color::Rgb(25, 23, 36),
                fg: Color::Rgb(224, 222, 244),
                accent: Color::Rgb(235, 188, 186),
                secondary_accent: Color::Rgb(196, 167, 231),
                border_active: Color::Rgb(235, 188, 186),
                border_inactive: Color::Rgb(82, 79, 103),
                text_muted: Color::Rgb(144, 140, 170),
                success: Color::Rgb(156, 207, 216),
                warning: Color::Rgb(246, 193, 119),
                error: Color::Rgb(235, 111, 146),
            },
            Self::CatppuccinMocha => Palette {
                bg: Color::Rgb(30, 30, 46),
                fg: Color::Rgb(205, 214, 244),
                accent: Color::Rgb(137, 180, 250),           // Blue
                secondary_accent: Color::Rgb(203, 166, 247), // Mauve
                border_active: Color::Rgb(137, 180, 250),
                border_inactive: Color::Rgb(100, 105, 125), // Surface2
                text_muted: Color::Rgb(166, 173, 200),      // Subtext0
                success: Color::Rgb(166, 227, 161),         // Green
                warning: Color::Rgb(249, 226, 175),         // Yellow
                error: Color::Rgb(243, 139, 168),           // Red
            },
            Self::Nord => Palette {
                bg: Color::Rgb(46, 52, 64),
                fg: Color::Rgb(216, 222, 233),
                accent: Color::Rgb(136, 192, 208),
                secondary_accent: Color::Rgb(129, 161, 193),
                border_active: Color::Rgb(136, 192, 208),
                border_inactive: Color::Rgb(100, 115, 135),
                text_muted: Color::Rgb(143, 188, 187),
                success: Color::Rgb(163, 190, 140),
                warning: Color::Rgb(235, 203, 139),
                error: Color::Rgb(191, 97, 106),
            },
            Self::TokyoNight => Palette {
                bg: Color::Rgb(26, 27, 38),
                fg: Color::Rgb(192, 202, 245),
                accent: Color::Rgb(122, 162, 247),
                secondary_accent: Color::Rgb(187, 154, 247),
                border_active: Color::Rgb(122, 162, 247),
                border_inactive: Color::Rgb(65, 72, 104),
                text_muted: Color::Rgb(86, 95, 137),
                success: Color::Rgb(158, 206, 106),
                warning: Color::Rgb(224, 175, 104),
                error: Color::Rgb(247, 118, 142),
            },
            Self::OneDark => Palette {
                bg: Color::Rgb(40, 44, 52),
                fg: Color::Rgb(171, 178, 191),
                accent: Color::Rgb(97, 175, 239),
                secondary_accent: Color::Rgb(198, 120, 221),
                border_active: Color::Rgb(97, 175, 239),
                border_inactive: Color::Rgb(105, 115, 135),
                text_muted: Color::Rgb(112, 120, 135),
                success: Color::Rgb(152, 195, 121),
                warning: Color::Rgb(229, 192, 123),
                error: Color::Rgb(224, 108, 117),
            },
            Self::AyuDark => Palette {
                bg: Color::Rgb(15, 20, 25),
                fg: Color::Rgb(230, 225, 207),
                accent: Color::Rgb(255, 180, 84),
                secondary_accent: Color::Rgb(54, 163, 217),
                border_active: Color::Rgb(255, 180, 84),
                border_inactive: Color::Rgb(62, 75, 89),
                text_muted: Color::Rgb(92, 103, 115),
                success: Color::Rgb(145, 181, 92),
                warning: Color::Rgb(255, 180, 84),
                error: Color::Rgb(240, 113, 120),
            },
            Self::AyuMirage => Palette {
                bg: Color::Rgb(33, 39, 51),
                fg: Color::Rgb(217, 215, 206),
                accent: Color::Rgb(255, 204, 102),
                secondary_accent: Color::Rgb(92, 207, 230),
                border_active: Color::Rgb(255, 204, 102),
                border_inactive: Color::Rgb(92, 103, 115),
                text_muted: Color::Rgb(112, 116, 143),
                success: Color::Rgb(186, 230, 126),
                warning: Color::Rgb(255, 204, 102),
                error: Color::Rgb(242, 135, 121),
            },
            Self::SolarizedDark => Palette {
                bg: Color::Rgb(0, 43, 54),
                fg: Color::Rgb(131, 148, 150),
                accent: Color::Rgb(38, 139, 210),
                secondary_accent: Color::Rgb(42, 161, 152),
                border_active: Color::Rgb(38, 139, 210),
                border_inactive: Color::Rgb(88, 110, 117),
                text_muted: Color::Rgb(88, 110, 117),
                success: Color::Rgb(133, 153, 0),
                warning: Color::Rgb(181, 137, 0),
                error: Color::Rgb(220, 50, 47),
            },
            Self::Everforest => Palette {
                bg: Color::Rgb(45, 53, 59),
                fg: Color::Rgb(211, 198, 170),
                accent: Color::Rgb(167, 192, 128),
                secondary_accent: Color::Rgb(127, 187, 161),
                border_active: Color::Rgb(167, 192, 128),
                border_inactive: Color::Rgb(100, 115, 120),
                text_muted: Color::Rgb(160, 175, 165),
                success: Color::Rgb(167, 192, 128),
                warning: Color::Rgb(219, 188, 127),
                error: Color::Rgb(230, 126, 128),
            },
            Self::Kanagawa => Palette {
                bg: Color::Rgb(31, 31, 40),
                fg: Color::Rgb(220, 215, 186),
                accent: Color::Rgb(126, 156, 216),
                secondary_accent: Color::Rgb(149, 127, 178),
                border_active: Color::Rgb(126, 156, 216),
                border_inactive: Color::Rgb(84, 84, 109),
                text_muted: Color::Rgb(114, 113, 105),
                success: Color::Rgb(118, 148, 106),
                warning: Color::Rgb(192, 163, 110),
                error: Color::Rgb(195, 64, 67),
            },
            Self::Zenburn => Palette {
                bg: Color::Rgb(63, 63, 63),
                fg: Color::Rgb(220, 220, 204),
                accent: Color::Rgb(140, 208, 211),
                secondary_accent: Color::Rgb(192, 190, 208),
                border_active: Color::Rgb(140, 208, 211),
                border_inactive: Color::Rgb(95, 127, 95),
                text_muted: Color::Rgb(127, 159, 127),
                success: Color::Rgb(127, 159, 127),
                warning: Color::Rgb(240, 223, 175),
                error: Color::Rgb(204, 147, 147),
            },
            Self::Monokai => Palette {
                bg: Color::Rgb(39, 40, 34),
                fg: Color::Rgb(248, 248, 242),
                accent: Color::Rgb(166, 226, 46),
                secondary_accent: Color::Rgb(102, 217, 239),
                border_active: Color::Rgb(166, 226, 46),
                border_inactive: Color::Rgb(117, 113, 94),
                text_muted: Color::Rgb(117, 113, 94),
                success: Color::Rgb(166, 226, 46),
                warning: Color::Rgb(253, 151, 31),
                error: Color::Rgb(249, 38, 114),
            },
            Self::NightOwl => Palette {
                bg: Color::Rgb(1, 22, 39),
                fg: Color::Rgb(214, 222, 235),
                accent: Color::Rgb(130, 170, 255),
                secondary_accent: Color::Rgb(199, 146, 234),
                border_active: Color::Rgb(130, 170, 255),
                border_inactive: Color::Rgb(45, 80, 120),
                text_muted: Color::Rgb(130, 160, 190),
                success: Color::Rgb(34, 218, 110),
                warning: Color::Rgb(255, 203, 139),
                error: Color::Rgb(239, 83, 80),
            },
            Self::MaterialOcean => Palette {
                bg: Color::Rgb(15, 17, 26),
                fg: Color::Rgb(143, 147, 162),
                accent: Color::Rgb(130, 170, 255),
                secondary_accent: Color::Rgb(199, 146, 234),
                border_active: Color::Rgb(130, 170, 255),
                border_inactive: Color::Rgb(70, 85, 100),
                text_muted: Color::Rgb(120, 130, 150),
                success: Color::Rgb(195, 232, 141),
                warning: Color::Rgb(255, 203, 107),
                error: Color::Rgb(240, 113, 120),
            },
            Self::Cyberpunk => Palette {
                bg: Color::Rgb(9, 6, 34),
                fg: Color::Rgb(249, 241, 165),
                accent: Color::Rgb(252, 238, 10),
                secondary_accent: Color::Rgb(0, 255, 255),
                border_active: Color::Rgb(252, 238, 10),
                border_inactive: Color::Rgb(255, 0, 60),
                text_muted: Color::Rgb(180, 160, 220),
                success: Color::Rgb(0, 255, 153),
                warning: Color::Rgb(255, 204, 0),
                error: Color::Rgb(255, 0, 60),
            },
            Self::Synthwave84 => Palette {
                bg: Color::Rgb(38, 35, 53),
                fg: Color::Rgb(255, 255, 255),
                accent: Color::Rgb(255, 126, 219),
                secondary_accent: Color::Rgb(54, 249, 246),
                border_active: Color::Rgb(255, 126, 219),
                border_inactive: Color::Rgb(97, 77, 133),
                text_muted: Color::Rgb(132, 139, 189),
                success: Color::Rgb(114, 241, 184),
                warning: Color::Rgb(254, 218, 51),
                error: Color::Rgb(254, 68, 80),
            },
            Self::HackerGreen => Palette {
                bg: Color::Rgb(0, 0, 0),
                fg: Color::Rgb(51, 255, 0),
                accent: Color::Rgb(0, 255, 0),
                secondary_accent: Color::Rgb(0, 200, 0),
                border_active: Color::Rgb(51, 255, 0),
                border_inactive: Color::Rgb(0, 100, 0),
                text_muted: Color::Rgb(0, 150, 0),
                success: Color::Rgb(51, 255, 0),
                warning: Color::Rgb(200, 255, 0),
                error: Color::Rgb(255, 50, 50),
            },
            Self::PhosphorBlue => Palette {
                bg: Color::Rgb(0, 0, 0),
                fg: Color::Rgb(0, 255, 255),
                accent: Color::Rgb(0, 255, 255),
                secondary_accent: Color::Rgb(0, 200, 255),
                border_active: Color::Rgb(0, 255, 255),
                border_inactive: Color::Rgb(0, 100, 150),
                text_muted: Color::Rgb(0, 150, 200),
                success: Color::Rgb(0, 255, 150),
                warning: Color::Rgb(200, 255, 0),
                error: Color::Rgb(255, 50, 50),
            },
            Self::Commodore64 => Palette {
                bg: Color::Rgb(64, 64, 224),
                fg: Color::Rgb(160, 160, 255),
                accent: Color::Rgb(255, 255, 255),
                secondary_accent: Color::Rgb(160, 160, 255),
                border_active: Color::Rgb(255, 255, 255),
                border_inactive: Color::Rgb(160, 160, 255),
                text_muted: Color::Rgb(160, 160, 255),
                success: Color::Rgb(255, 255, 255),
                warning: Color::Rgb(255, 255, 255),
                error: Color::Rgb(255, 0, 0),
            },
            Self::Grayscale => Palette {
                bg: Color::Rgb(20, 20, 20),
                fg: Color::Rgb(230, 230, 230),
                accent: Color::Rgb(255, 255, 255),
                secondary_accent: Color::Rgb(180, 180, 180),
                border_active: Color::Rgb(255, 255, 255),
                border_inactive: Color::Rgb(100, 100, 100),
                text_muted: Color::Rgb(130, 130, 130),
                success: Color::Rgb(255, 255, 255),
                warning: Color::Rgb(200, 200, 200),
                error: Color::Rgb(100, 100, 100),
            },
            // Mac OS Platinum: black ink on light gray, with the 1977–1998
            // rainbow Apple logo colours (#61BB46 #FDB827 #F5821F #E03A3E
            // #963D97 #009DDC). Text roles use darker steps of the logo
            // colours so they stay legible on the light background.
            Self::ClassicMacintosh => Palette {
                bg: Color::Rgb(0xDD, 0xDD, 0xDD),               // Platinum gray
                fg: Color::Rgb(0x00, 0x00, 0x00),               // Black ink
                accent: Color::Rgb(0x96, 0x3D, 0x97),           // Logo purple
                secondary_accent: Color::Rgb(0x00, 0x74, 0xA6), // Logo blue, darkened
                border_active: Color::Rgb(0x00, 0x00, 0x00),
                border_inactive: Color::Rgb(0x88, 0x88, 0x88),
                text_muted: Color::Rgb(0x55, 0x55, 0x55),
                success: Color::Rgb(0x3A, 0x7D, 0x28), // Logo green, darkened
                warning: Color::Rgb(0xB8, 0x5A, 0x0E), // Logo orange, darkened
                error: Color::Rgb(0xC0, 0x2A, 0x2E),   // Logo red, darkened
            },
            // A reflective LCD watch face: dark segments on a gray-green
            // panel, with Casio logo blue (#003296) and the gold lettering of
            // metal-band models.
            Self::CasioDigital => Palette {
                bg: Color::Rgb(0xB9, 0xC2, 0xA4),               // LCD panel
                fg: Color::Rgb(0x1C, 0x22, 0x1A),               // LCD segment
                accent: Color::Rgb(0x00, 0x32, 0x96),           // Casio blue
                secondary_accent: Color::Rgb(0x6E, 0x55, 0x12), // Gold lettering, darkened
                border_active: Color::Rgb(0x1C, 0x22, 0x1A),
                border_inactive: Color::Rgb(0x83, 0x8C, 0x74), // Unlit segment ghost
                text_muted: Color::Rgb(0x4A, 0x53, 0x44),
                success: Color::Rgb(0x2D, 0x5E, 0x2A),
                warning: Color::Rgb(0x7A, 0x4B, 0x00),
                error: Color::Rgb(0xA0, 0x22, 0x28),
            },
            // Nothing: black, white, and the brand red (#D71921).
            Self::Nothing => Palette {
                bg: Color::Rgb(0x00, 0x00, 0x00),
                fg: Color::Rgb(0xFF, 0xFF, 0xFF),
                accent: Color::Rgb(0xD7, 0x19, 0x21), // Nothing red
                secondary_accent: Color::Rgb(0xB3, 0xB3, 0xB3),
                border_active: Color::Rgb(0xFF, 0xFF, 0xFF),
                border_inactive: Color::Rgb(0x3A, 0x3A, 0x3A),
                text_muted: Color::Rgb(0x8A, 0x8A, 0x8A),
                success: Color::Rgb(0xE6, 0xE6, 0xE6),
                warning: Color::Rgb(0xF5, 0xC4, 0x00),
                error: Color::Rgb(0xD7, 0x19, 0x21),
            },
            // ThinkPad: matte black chassis, the TrackPoint red (Lenovo red
            // #E42022), and IBM blue for secondary emphasis.
            Self::ThinkPad => Palette {
                bg: Color::Rgb(0x12, 0x12, 0x12),               // Chassis black
                fg: Color::Rgb(0xE6, 0xE6, 0xE6),               // Keycap legend
                accent: Color::Rgb(0xE4, 0x20, 0x22),           // TrackPoint red
                secondary_accent: Color::Rgb(0x45, 0x89, 0xFF), // IBM blue, lightened
                border_active: Color::Rgb(0xE4, 0x20, 0x22),
                border_inactive: Color::Rgb(0x3D, 0x3D, 0x3D), // Keyboard gray
                text_muted: Color::Rgb(0x8D, 0x8D, 0x8D),
                success: Color::Rgb(0x42, 0xBE, 0x65),
                warning: Color::Rgb(0xF1, 0xC2, 0x1B),
                error: Color::Rgb(0xFA, 0x4D, 0x56),
            },
        }
    }

    #[must_use]
    pub fn styles(self) -> SemanticStyles {
        self.palette().styles()
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
    #[allow(clippy::too_many_lines)]
    fn test_semantic_styles_pure_derivation_from_palette() {
        let palette = Theme::Amber.palette();
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
        let styles = Theme::Amber.styles();

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
        let c64 = Theme::Commodore64.palette();
        assert_eq!(c64.success, c64.warning);

        // Grayscale uses white for accent, border_active, and success
        let gray = Theme::Grayscale.palette();
        assert_eq!(gray.accent, gray.border_active);
        assert_eq!(gray.accent, gray.success);

        // HackerGreen uses pure green for fg, border_active, and success
        let green = Theme::HackerGreen.palette();
        assert_eq!(green.fg, green.border_active);
        assert_eq!(green.fg, green.success);

        // Every theme resolves valid semantic styles without panicking
        for theme in Theme::ALL {
            let styles = theme.styles();
            assert_ne!(styles.palette.bg, styles.palette.fg);
        }
    }

    #[test]
    fn test_daemon_status_derivation() {
        use crate::tui::model::DaemonLifecycle;

        let styles = Theme::Amber.styles();

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
        let palette = Theme::Dracula.palette();
        let from_ref = SemanticStyles::from(&palette);
        let from_val = SemanticStyles::from(palette);
        let from_theme = Theme::Dracula.styles();

        assert_eq!(from_ref, from_val);
        assert_eq!(from_ref, from_theme);
    }
}
