//! Theme registry tests. No test here reads the user's real config dir.

use std::path::PathBuf;

use super::*;

type LegacyRow = (
    &'static str,
    &'static str,
    &'static str,
    bool,
    [(u8, u8, u8); 10],
);

/// Frozen copy of the pre-registry `tui::theme::Theme` table, generated
/// mechanically from `Theme::palette()`, `Theme::name()` and
/// `Theme::glyph_style()` before those match arms were deleted.
///
/// `(id, legacy serde name, display name, pixel glyphs, palette in
/// Palette::KEYS order)`.
const LEGACY_THEMES: [LegacyRow; 35] = [
    (
        "amber",
        "amber",
        "Amber",
        false,
        [
            (15, 10, 5),
            (255, 210, 150),
            (255, 176, 0),
            (220, 140, 0),
            (255, 176, 0),
            (120, 80, 20),
            (180, 120, 50),
            (170, 220, 100),
            (255, 176, 0),
            (255, 60, 60),
        ],
    ),
    (
        "amiga",
        "amiga",
        "Amiga",
        true,
        [
            (20, 24, 32),
            (240, 244, 250),
            (238, 34, 51),
            (85, 136, 187),
            (255, 136, 0),
            (40, 50, 68),
            (114, 134, 158),
            (63, 191, 95),
            (255, 136, 0),
            (221, 51, 34),
        ],
    ),
    (
        "ayudark",
        "ayudark",
        "Ayu Dark",
        false,
        [
            (15, 20, 25),
            (230, 225, 207),
            (255, 180, 84),
            (54, 163, 217),
            (255, 180, 84),
            (62, 75, 89),
            (92, 103, 115),
            (145, 181, 92),
            (255, 180, 84),
            (240, 113, 120),
        ],
    ),
    (
        "ayumirage",
        "ayumirage",
        "Ayu Mirage",
        false,
        [
            (33, 39, 51),
            (217, 215, 206),
            (255, 204, 102),
            (92, 207, 230),
            (255, 204, 102),
            (92, 103, 115),
            (112, 116, 143),
            (186, 230, 126),
            (255, 204, 102),
            (242, 135, 121),
        ],
    ),
    (
        "braun",
        "braun",
        "Braun",
        false,
        [
            (22, 22, 22),
            (242, 242, 240),
            (255, 85, 0),
            (240, 176, 30),
            (255, 85, 0),
            (64, 64, 64),
            (138, 135, 129),
            (36, 138, 61),
            (217, 119, 6),
            (220, 38, 38),
        ],
    ),
    (
        "casiodigital",
        "casiodigital",
        "Casio Digital",
        false,
        [
            (18, 24, 20),
            (224, 248, 220),
            (0, 136, 255),
            (136, 166, 132),
            (157, 240, 154),
            (42, 56, 46),
            (96, 122, 94),
            (157, 240, 154),
            (242, 208, 107),
            (255, 82, 82),
        ],
    ),
    (
        "catppuccinmocha",
        "catppuccinmocha",
        "Catppuccin Mocha",
        false,
        [
            (30, 30, 46),
            (205, 214, 244),
            (137, 180, 250),
            (203, 166, 247),
            (137, 180, 250),
            (100, 105, 125),
            (166, 173, 200),
            (166, 227, 161),
            (249, 226, 175),
            (243, 139, 168),
        ],
    ),
    (
        "classicmacintosh",
        "classicmacintosh",
        "Classic Macintosh",
        false,
        [
            (18, 18, 18),
            (242, 242, 236),
            (0, 157, 220),
            (150, 61, 151),
            (242, 242, 236),
            (62, 62, 62),
            (136, 136, 130),
            (97, 187, 70),
            (253, 184, 39),
            (224, 58, 62),
        ],
    ),
    (
        "commodore64",
        "commodore64",
        "Commodore 64",
        true,
        [
            (16, 18, 37),
            (167, 183, 255),
            (255, 255, 255),
            (127, 229, 229),
            (255, 255, 255),
            (59, 63, 120),
            (102, 114, 168),
            (255, 255, 255),
            (255, 255, 255),
            (255, 0, 0),
        ],
    ),
    (
        "cyberpunk",
        "cyberpunk",
        "Cyberpunk",
        true,
        [
            (9, 6, 34),
            (249, 241, 165),
            (252, 238, 10),
            (0, 255, 255),
            (252, 238, 10),
            (255, 0, 60),
            (180, 160, 220),
            (0, 255, 153),
            (255, 204, 0),
            (255, 0, 60),
        ],
    ),
    (
        "dracula",
        "dracula",
        "Dracula",
        false,
        [
            (40, 42, 54),
            (248, 248, 242),
            (255, 121, 198),
            (189, 147, 249),
            (255, 121, 198),
            (98, 114, 164),
            (191, 191, 191),
            (80, 250, 123),
            (241, 250, 140),
            (255, 85, 85),
        ],
    ),
    (
        "everforest",
        "everforest",
        "Everforest",
        false,
        [
            (45, 53, 59),
            (211, 198, 170),
            (167, 192, 128),
            (127, 187, 161),
            (167, 192, 128),
            (100, 115, 120),
            (160, 175, 165),
            (167, 192, 128),
            (219, 188, 127),
            (230, 126, 128),
        ],
    ),
    (
        "grayscale",
        "grayscale",
        "Grayscale",
        false,
        [
            (20, 20, 20),
            (230, 230, 230),
            (255, 255, 255),
            (180, 180, 180),
            (255, 255, 255),
            (100, 100, 100),
            (130, 130, 130),
            (255, 255, 255),
            (200, 200, 200),
            (100, 100, 100),
        ],
    ),
    (
        "gruvbox",
        "gruvbox",
        "Gruvbox",
        false,
        [
            (40, 40, 40),
            (235, 219, 178),
            (215, 153, 33),
            (204, 36, 29),
            (215, 153, 33),
            (102, 92, 84),
            (168, 153, 132),
            (152, 151, 26),
            (215, 153, 33),
            (204, 36, 29),
        ],
    ),
    (
        "hackergreen",
        "hackergreen",
        "Hacker Green",
        true,
        [
            (0, 0, 0),
            (51, 255, 0),
            (0, 255, 0),
            (0, 200, 0),
            (51, 255, 0),
            (0, 100, 0),
            (0, 150, 0),
            (51, 255, 0),
            (200, 255, 0),
            (255, 50, 50),
        ],
    ),
    (
        "handheld",
        "handheld",
        "Handheld",
        true,
        [
            (20, 28, 18),
            (155, 188, 15),
            (139, 29, 66),
            (139, 172, 15),
            (155, 188, 15),
            (40, 56, 36),
            (86, 118, 72),
            (139, 172, 15),
            (200, 166, 70),
            (154, 34, 87),
        ],
    ),
    (
        "kanagawa",
        "kanagawa",
        "Kanagawa",
        false,
        [
            (31, 31, 40),
            (220, 215, 186),
            (126, 156, 216),
            (149, 127, 178),
            (126, 156, 216),
            (84, 84, 109),
            (114, 113, 105),
            (118, 148, 106),
            (192, 163, 110),
            (195, 64, 67),
        ],
    ),
    (
        "materialocean",
        "materialocean",
        "Material Ocean",
        false,
        [
            (15, 17, 26),
            (143, 147, 162),
            (130, 170, 255),
            (199, 146, 234),
            (130, 170, 255),
            (70, 85, 100),
            (120, 130, 150),
            (195, 232, 141),
            (255, 203, 107),
            (240, 113, 120),
        ],
    ),
    (
        "monokai",
        "monokai",
        "Monokai",
        false,
        [
            (39, 40, 34),
            (248, 248, 242),
            (166, 226, 46),
            (102, 217, 239),
            (166, 226, 46),
            (117, 113, 94),
            (117, 113, 94),
            (166, 226, 46),
            (253, 151, 31),
            (249, 38, 114),
        ],
    ),
    (
        "nightowl",
        "nightowl",
        "Night Owl",
        false,
        [
            (1, 22, 39),
            (214, 222, 235),
            (130, 170, 255),
            (199, 146, 234),
            (130, 170, 255),
            (45, 80, 120),
            (130, 160, 190),
            (34, 218, 110),
            (255, 203, 139),
            (239, 83, 80),
        ],
    ),
    (
        "nixie",
        "nixie",
        "Nixie",
        false,
        [
            (11, 10, 9),
            (255, 183, 101),
            (255, 122, 24),
            (226, 78, 5),
            (255, 122, 24),
            (57, 51, 44),
            (140, 133, 122),
            (73, 209, 122),
            (255, 176, 32),
            (255, 68, 51),
        ],
    ),
    (
        "nord",
        "nord",
        "Nord",
        false,
        [
            (46, 52, 64),
            (216, 222, 233),
            (136, 192, 208),
            (129, 161, 193),
            (136, 192, 208),
            (100, 115, 135),
            (143, 188, 187),
            (163, 190, 140),
            (235, 203, 139),
            (191, 97, 106),
        ],
    ),
    (
        "nothing",
        "nothing",
        "Nothing",
        false,
        [
            (0, 0, 0),
            (255, 255, 255),
            (215, 25, 33),
            (179, 179, 179),
            (255, 255, 255),
            (58, 58, 58),
            (138, 138, 138),
            (230, 230, 230),
            (245, 196, 0),
            (215, 25, 33),
        ],
    ),
    (
        "onedark",
        "onedark",
        "One Dark",
        false,
        [
            (40, 44, 52),
            (171, 178, 191),
            (97, 175, 239),
            (198, 120, 221),
            (97, 175, 239),
            (105, 115, 135),
            (112, 120, 135),
            (152, 195, 121),
            (229, 192, 123),
            (224, 108, 117),
        ],
    ),
    (
        "oscilloscope",
        "oscilloscope",
        "Oscilloscope",
        false,
        [
            (5, 9, 6),
            (120, 255, 120),
            (57, 255, 20),
            (0, 221, 85),
            (57, 255, 20),
            (20, 53, 27),
            (46, 139, 62),
            (120, 255, 120),
            (246, 163, 84),
            (255, 97, 94),
        ],
    ),
    (
        "phosphorblue",
        "phosphorblue",
        "Phosphor Blue",
        false,
        [
            (0, 0, 0),
            (0, 255, 255),
            (0, 255, 255),
            (0, 200, 255),
            (0, 255, 255),
            (0, 100, 150),
            (0, 150, 200),
            (0, 255, 150),
            (200, 255, 0),
            (255, 50, 50),
        ],
    ),
    (
        "rosepine",
        "rosepine",
        "Rosé Pine",
        false,
        [
            (25, 23, 36),
            (224, 222, 244),
            (235, 188, 186),
            (196, 167, 231),
            (235, 188, 186),
            (82, 79, 103),
            (144, 140, 170),
            (156, 207, 216),
            (246, 193, 119),
            (235, 111, 146),
        ],
    ),
    (
        "solarizeddark",
        "solarizeddark",
        "Solarized Dark",
        false,
        [
            (0, 43, 54),
            (131, 148, 150),
            (38, 139, 210),
            (42, 161, 152),
            (38, 139, 210),
            (88, 110, 117),
            (88, 110, 117),
            (133, 153, 0),
            (181, 137, 0),
            (220, 50, 47),
        ],
    ),
    (
        "synthwave84",
        "synthwave84",
        "Synthwave '84",
        true,
        [
            (38, 35, 53),
            (255, 255, 255),
            (255, 126, 219),
            (54, 249, 246),
            (255, 126, 219),
            (97, 77, 133),
            (132, 139, 189),
            (114, 241, 184),
            (254, 218, 51),
            (254, 68, 80),
        ],
    ),
    (
        "terminal",
        "terminal",
        "Terminal",
        false,
        [
            (26, 26, 26),
            (80, 200, 120),
            (140, 255, 170),
            (60, 180, 100),
            (80, 200, 120),
            (40, 100, 60),
            (60, 150, 90),
            (80, 200, 120),
            (200, 200, 0),
            (255, 80, 80),
        ],
    ),
    (
        "thinkpad",
        "thinkpad",
        "ThinkPad",
        false,
        [
            (18, 18, 18),
            (230, 230, 230),
            (228, 32, 34),
            (69, 137, 255),
            (228, 32, 34),
            (61, 61, 61),
            (141, 141, 141),
            (66, 190, 101),
            (241, 194, 27),
            (250, 77, 86),
        ],
    ),
    (
        "tokyonight",
        "tokyonight",
        "Tokyo Night",
        false,
        [
            (26, 27, 38),
            (192, 202, 245),
            (122, 162, 247),
            (187, 154, 247),
            (122, 162, 247),
            (65, 72, 104),
            (86, 95, 137),
            (158, 206, 106),
            (224, 175, 104),
            (247, 118, 142),
        ],
    ),
    (
        "unixworkstation",
        "unixworkstation",
        "UNIX Workstation",
        false,
        [
            (24, 29, 38),
            (241, 245, 249),
            (0, 240, 255),
            (56, 232, 255),
            (0, 240, 255),
            (74, 86, 110),
            (100, 116, 139),
            (0, 230, 118),
            (255, 176, 32),
            (255, 51, 68),
        ],
    ),
    (
        "vfd_hifi",
        "vfdhifi",
        "VFD Hi-Fi",
        false,
        [
            (11, 14, 17),
            (0, 240, 168),
            (102, 255, 212),
            (27, 178, 137),
            (0, 240, 168),
            (24, 40, 36),
            (82, 122, 112),
            (0, 240, 168),
            (255, 176, 0),
            (255, 59, 48),
        ],
    ),
    (
        "zenburn",
        "zenburn",
        "Zenburn",
        false,
        [
            (63, 63, 63),
            (220, 220, 204),
            (140, 208, 211),
            (192, 190, 208),
            (140, 208, 211),
            (95, 127, 95),
            (127, 159, 127),
            (127, 159, 127),
            (240, 223, 175),
            (204, 147, 147),
        ],
    ),
];

/// Legacy `#[serde(alias = ...)]` spellings of the old `config::Theme` enum.
const LEGACY_ALIASES: [(&str, &str); 23] = [
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

fn user(stem: &str, text: &str) -> UserThemeSource {
    UserThemeSource {
        stem: stem.to_owned(),
        path: PathBuf::from(format!("/virtual/themes/{stem}.toml")),
        text: Ok(text.to_owned()),
    }
}

fn errors_for<'a>(registry: &'a Registry, stem: &str) -> &'a [ThemeError] {
    registry
        .diagnostics()
        .iter()
        .find(|diagnostic| diagnostic.stem == stem)
        .map_or(&[], |diagnostic| diagnostic.errors.as_slice())
}

const MINIMAL: &str = r##"
id = "mine"
name = "Mine"
family = "terminal"

[palette]
bg = "#000000"
fg = "#ffffff"
accent = "#ff0000"
secondary_accent = "#00ff00"
border_active = "#0000ff"
border_inactive = "#111111"
text_muted = "#888888"
success = "#00ff00"
warning = "#ffff00"
error = "#ff0000"
"##;

#[test]
fn builtins_load_cleanly_and_sorted() {
    let registry = Registry::builtin();
    assert!(
        registry.diagnostics().is_empty(),
        "{:?}",
        registry.diagnostics()
    );
    assert_eq!(registry.themes().len(), 40);
    let ids: Vec<&str> = registry.themes().iter().map(|t| t.id.as_str()).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_eq!(ids, sorted);
    let added = ["ps3", "winxp", "macos", "wii", "nds"];
    assert_eq!(
        ids.iter()
            .copied()
            .filter(|id| !added.contains(id))
            .collect::<Vec<_>>(),
        LEGACY_THEMES.iter().map(|row| row.0).collect::<Vec<_>>(),
        "registry order must match the legacy Theme::ALL order"
    );
    assert!(registry.get(DEFAULT_THEME_ID).is_some());
}

#[test]
fn builtin_palettes_match_frozen_legacy_table() {
    let registry = Registry::builtin();
    for (id, _, name, pixel, colours) in LEGACY_THEMES {
        let theme = registry.get(id).unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(theme.name, name, "{id} name");
        let expected_glyph = if pixel {
            GlyphStyle::Pixel
        } else {
            GlyphStyle::Rounded
        };
        assert_eq!(theme.tui.glyph_style, expected_glyph, "{id} glyph style");
        for ((key, actual), (r, g, b)) in theme.palette.entries().into_iter().zip(colours) {
            assert_eq!(actual, Rgb::new(r, g, b), "{id} palette.{key}");
        }
    }
}

#[test]
fn builtin_families_follow_design_classification() {
    let pixel = [
        "handheld",
        "commodore64",
        "amiga",
        "classicmacintosh",
        "nds",
    ];
    let instrument = [
        "braun",
        "vfd_hifi",
        "nixie",
        "oscilloscope",
        "thinkpad",
        "unixworkstation",
        "phosphorblue",
        "amber",
        "grayscale",
        "nothing",
        "casiodigital",
        "cyberpunk",
        "synthwave84",
        "ps3",
        "winxp",
        "macos",
        "wii",
    ];
    let registry = Registry::builtin();
    for theme in registry.themes() {
        let expected = if pixel.contains(&theme.id.as_str()) {
            Family::Pixel
        } else if instrument.contains(&theme.id.as_str()) {
            Family::Instrument
        } else {
            Family::Terminal
        };
        assert_eq!(theme.family, expected, "{}", theme.id);
    }
    assert_eq!(
        registry
            .themes()
            .iter()
            .filter(|t| t.family == Family::Terminal)
            .count(),
        18
    );
}

#[test]
fn every_legacy_config_value_still_resolves() {
    let registry = Registry::builtin();
    for (id, serde_name, ..) in LEGACY_THEMES {
        assert_eq!(registry.canonical_id(serde_name), Some(id), "{serde_name}");
        assert_eq!(registry.canonical_id(id), Some(id));
    }
    for (alias, id) in LEGACY_ALIASES {
        assert_eq!(registry.canonical_id(alias), Some(id), "{alias}");
    }
}

#[test]
fn resolution_is_case_and_separator_tolerant() {
    let registry = Registry::builtin();
    assert_eq!(registry.canonical_id("  Dracula "), Some("dracula"));
    assert_eq!(registry.canonical_id("VFD-HiFi"), Some("vfd_hifi"));
    assert_eq!(
        registry.canonical_id("catppuccin_mocha"),
        Some("catppuccinmocha")
    );
    assert_eq!(registry.canonical_id("Tokyo Night"), Some("tokyonight"));
    assert!(registry.resolve("").is_err());
}

#[test]
fn unknown_names_suggest_nearest_matches() {
    let registry = Registry::builtin();
    let error = registry.resolve("dracla").unwrap_err();
    assert_eq!(
        error.suggestions.first().map(String::as_str),
        Some("dracula")
    );
    assert!(
        error.to_string().contains("did you mean `dracula`"),
        "{error}"
    );

    let error = registry.resolve("gameboi").unwrap_err();
    assert_eq!(
        error.suggestions.first().map(String::as_str),
        Some("handheld")
    );

    let error = registry.resolve("solarized").unwrap_err();
    assert!(error.suggestions.contains(&String::from("solarizeddark")));
    assert!(error.suggestions.len() <= 3);

    assert!(registry
        .resolve("qqqqqqqqqqqq")
        .unwrap_err()
        .suggestions
        .is_empty());
}

#[test]
fn family_defaults_fill_widget_tokens() {
    let registry = Registry::builtin();
    let handheld = registry.get("handheld").unwrap();
    assert_eq!(handheld.widget.renderer, Renderer::Pixel);
    assert_eq!(handheld.widget.font, Font::Bitmap8);
    assert_eq!(handheld.widget.dither, Dither::Bayer4);
    assert_eq!(handheld.widget.density, Density::Compact);
    assert_eq!(handheld.widget.radius, 0);
    assert_eq!(handheld.widget.pixel_scale, 0);
    assert_eq!(handheld.widget.modules, Module::ALL.to_vec());

    let braun = registry.get("braun").unwrap();
    assert_eq!(braun.widget.renderer, Renderer::Vector);
    assert_eq!(braun.widget.font, Font::Sans);
    assert_eq!(braun.widget.radius, 4);
    assert_eq!(braun.widget.density, Density::Comfortable);

    let nothing = registry.get("nothing").unwrap();
    assert_eq!(nothing.family, Family::Instrument);
    assert_eq!(nothing.widget.renderer, Renderer::Vector);
    assert_eq!(nothing.widget.font, Font::Mono);
    assert_eq!(nothing.widget.radius, 24);
    assert_eq!(nothing.widget.border, Border::Line);
    assert_eq!(nothing.widget.density, Density::Compact);
    assert_eq!(nothing.widget.glow, 0.0);

    let nord = registry.get("nord").unwrap();
    assert_eq!(nord.widget.renderer, Renderer::Terminal);
    assert_eq!(nord.widget.font, Font::Mono);
    assert_eq!(nord.widget.border, Border::Line);

    for theme in registry.themes() {
        let shades = &theme.widget.shades;
        assert_eq!(shades.len(), 4, "{}", theme.id);
        assert!(
            shades
                .windows(2)
                .all(|w| w[0].luminance() <= w[1].luminance()),
            "{} shades must run dark to light",
            theme.id
        );
        assert!(shades.contains(&theme.palette.bg) && shades.contains(&theme.palette.fg));
    }
}

#[test]
fn user_theme_shadows_builtin_and_aliases_follow_the_id() {
    let text = MINIMAL.replace("id = \"mine\"", "id = \"amiga\"");
    let registry = Registry::with_user_sources(vec![user("amiga", &text)]);
    assert!(
        registry.diagnostics().is_empty(),
        "{:?}",
        registry.diagnostics()
    );
    assert_eq!(registry.themes().len(), 40);
    let amiga = registry.get("amiga").unwrap();
    assert_eq!(amiga.name, "Mine");
    assert!(matches!(amiga.source, ThemeSource::User(_)));
    // The built-in alias still points at the id, now the user's theme.
    assert_eq!(registry.resolve("workbench").unwrap().name, "Mine");
}

#[test]
fn user_theme_extends_builtin_and_overrides_only_what_it_sets() {
    let registry = Registry::with_user_sources(vec![user(
        "dusk",
        r##"
id = "dusk"
name = "Dusk"
extends = "dracula"
family = "pixel"

[palette]
accent = "#123456"
"##,
    )]);
    assert!(
        registry.diagnostics().is_empty(),
        "{:?}",
        registry.diagnostics()
    );
    let dusk = registry.get("dusk").unwrap();
    let dracula = registry.get("dracula").unwrap();
    assert_eq!(dusk.palette.accent, Rgb::new(0x12, 0x34, 0x56));
    assert_eq!(dusk.palette.bg, dracula.palette.bg);
    assert_eq!(dusk.palette.error, dracula.palette.error);
    assert_eq!(dusk.extends.as_deref(), Some("dracula"));
    // Widget defaults come from the child's family, not the base's.
    assert_eq!(dusk.family, Family::Pixel);
    assert_eq!(dusk.widget.renderer, Renderer::Pixel);
    assert!(dusk.aliases.is_empty());
    assert_eq!(registry.themes().len(), 41);
}

#[test]
fn extends_inherits_name_and_own_id_means_builtin() {
    let registry = Registry::with_user_sources(vec![user(
        "nord",
        "id = \"nord\"\nextends = \"nord\"\n[palette]\nbg = \"#000000\"\n",
    )]);
    assert!(
        registry.diagnostics().is_empty(),
        "{:?}",
        registry.diagnostics()
    );
    let nord = registry.get("nord").unwrap();
    assert_eq!(nord.name, "Nord");
    assert_eq!(nord.palette.bg, Rgb::new(0, 0, 0));
    assert_eq!(
        nord.palette.fg,
        Registry::builtin().get("nord").unwrap().palette.fg
    );
}

#[test]
fn extends_chains_through_user_themes_and_detects_cycles() {
    let registry = Registry::with_user_sources(vec![
        user(
            "a",
            "id = \"a\"\nname = \"A\"\nextends = \"b\"\n[palette]\nbg = \"#010101\"\n",
        ),
        user(
            "b",
            "id = \"b\"\nname = \"B\"\nextends = \"gruvbox\"\n[palette]\nfg = \"#020202\"\n",
        ),
        user("x", "id = \"x\"\nname = \"X\"\nextends = \"y\"\n"),
        user("y", "id = \"y\"\nname = \"Y\"\nextends = \"x\"\n"),
    ]);
    let a = registry.get("a").unwrap();
    assert_eq!(a.palette.bg, Rgb::new(1, 1, 1));
    assert_eq!(a.palette.fg, Rgb::new(2, 2, 2));
    assert_eq!(a.family, Family::Terminal);
    assert!(registry.get("x").is_none());
    assert!(matches!(
        errors_for(&registry, "x"),
        [ThemeError::ExtendsCycle { .. }]
    ));
}

#[test]
fn unknown_extends_reports_suggestions() {
    let registry = Registry::with_user_sources(vec![user(
        "t",
        "id = \"t\"\nname = \"T\"\nextends = \"draculla\"\n",
    )]);
    match errors_for(&registry, "t") {
        [ThemeError::UnknownExtends {
            extends,
            suggestions,
        }] => {
            assert_eq!(extends, "draculla");
            assert_eq!(suggestions.first().map(String::as_str), Some("dracula"));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn invalid_files_report_every_structured_error() {
    let text = MINIMAL
        .replace("id = \"mine\"", "id = \"other\"")
        .replace("family = \"terminal\"", "family = \"retro\"")
        .replace("fg = \"#ffffff\"", "fg = \"white\"")
        .replace("error = \"#ff0000\"\n", "");
    let text = format!("{text}\n[widget]\nradius = 99\nmodules = [\"telemetry\", \"nope\"]\n");
    let registry = Registry::with_user_sources(vec![user("mine", &text)]);
    assert!(registry.get("mine").is_none() && registry.get("other").is_none());
    let errors = errors_for(&registry, "mine");
    assert!(errors.contains(&ThemeError::IdMismatch {
        id: String::from("other"),
        file_stem: String::from("mine"),
    }));
    assert!(errors.contains(&ThemeError::UnknownFamily {
        value: String::from("retro"),
    }));
    assert!(errors.contains(&ThemeError::BadHex {
        field: String::from("palette.fg"),
        value: String::from("white"),
    }));
    assert!(errors.contains(&ThemeError::MissingPaletteKey { key: "error" }));
    assert!(errors
        .iter()
        .any(|e| matches!(e, ThemeError::InvalidValue { field, .. } if field == "widget.radius")));
    assert!(errors.iter().any(|e| matches!(
        e,
        ThemeError::InvalidValue { field, value, .. }
            if field == "widget.modules" && value == "nope"
    )));
    let rendered = registry.diagnostics()[0].to_string();
    assert!(rendered.contains("/virtual/themes/mine.toml"), "{rendered}");
}

#[test]
fn unknown_keys_and_bad_types_are_parse_errors() {
    let registry = Registry::with_user_sources(vec![
        user(
            "k",
            &format!("{}\nsparkle = true\n", MINIMAL.replace("\"mine\"", "\"k\"")),
        ),
        user(
            "p",
            &MINIMAL
                .replace("\"mine\"", "\"p\"")
                .replace("[palette]", "[palette]\nhover = \"#000000\""),
        ),
        user("s", "id = 5\n"),
    ]);
    for stem in ["k", "p", "s"] {
        assert!(
            matches!(errors_for(&registry, stem), [ThemeError::Parse { .. }]),
            "{stem}: {:?}",
            errors_for(&registry, stem)
        );
    }
    assert!(registry.diagnostics().iter().all(|d| d.path.is_some()));
}

#[test]
fn hex_parsing_is_strict() {
    assert_eq!(Rgb::parse_hex("#A0b1C2"), Some(Rgb::new(0xa0, 0xb1, 0xc2)));
    for bad in [
        "a0b1c2", "#a0b1c", "#a0b1c2d", "#g0b1c2", "", "#", "#+1b1c2",
    ] {
        assert_eq!(Rgb::parse_hex(bad), None, "{bad}");
    }
    assert_eq!(Rgb::new(1, 2, 255).to_hex(), "#0102ff");
}

#[test]
fn alias_conflicts_are_rejected() {
    let text = MINIMAL.replace("family", "aliases = [\"gameboy\"]\nfamily");
    let registry = Registry::with_user_sources(vec![user("mine", &text)]);
    assert!(matches!(
        errors_for(&registry, "mine"),
        [ThemeError::AliasConflict { alias, owner }] if alias == "gameboy" && owner == "handheld"
    ));
    assert_eq!(registry.canonical_id("gameboy"), Some("handheld"));

    // A user id equal to a built-in alias takes the name over.
    let text = MINIMAL.replace("\"mine\"", "\"gameboy\"");
    let registry = Registry::with_user_sources(vec![user("gameboy", &text)]);
    assert_eq!(registry.canonical_id("gameboy"), Some("gameboy"));
    assert_eq!(registry.canonical_id("dmg"), Some("handheld"));
}

#[test]
fn invalid_user_file_is_named_in_unknown_theme_errors() {
    let registry = Registry::with_user_sources(vec![user("mine", "id = \"mine\"\n")]);
    let error = registry.resolve("mine").unwrap_err();
    assert!(error.invalid_file.is_some());
    assert!(
        error.to_string().contains("name: required key is missing"),
        "{error}"
    );
}

#[test]
fn export_round_trips_every_builtin() {
    let builtin = Registry::builtin();
    for theme in builtin.themes() {
        let text = theme.to_toml();
        let registry = Registry::with_user_sources(vec![user(&theme.id, &text)]);
        assert!(
            registry.diagnostics().is_empty(),
            "{}: {:?}\n{text}",
            theme.id,
            registry.diagnostics()
        );
        let reloaded = registry.get(&theme.id).unwrap();
        assert_eq!(reloaded.name, theme.name);
        assert_eq!(reloaded.aliases, theme.aliases);
        assert_eq!(reloaded.family, theme.family);
        assert_eq!(reloaded.palette, theme.palette);
        assert_eq!(reloaded.tui, theme.tui);
        assert_eq!(reloaded.widget, theme.widget);
    }
}

#[test]
fn fixture_user_theme_loads_from_directory() {
    let dir = PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/themes"
    ));
    let registry = Registry::load_dir(&dir);
    assert!(
        registry.diagnostics().is_empty(),
        "{:?}",
        registry.diagnostics()
    );
    let theme = registry.resolve("gbp_night").unwrap();
    assert_eq!(theme.id, "pocket_night");
    assert_eq!(theme.family, Family::Pixel);
    assert_eq!(
        theme.palette.fg,
        Registry::builtin().get("handheld").unwrap().palette.fg
    );
    assert_eq!(theme.palette.accent, Rgb::new(0xc2, 0x3b, 0x5f));
    assert_eq!(theme.tui.glyph_style, GlyphStyle::Pixel);
    assert!((theme.widget.glow - 0.25).abs() < f64::EPSILON);
    assert_eq!(theme.widget.modules.len(), 4);

    let validated = validate_file(&dir.join("pocket_night.toml"), None).unwrap();
    assert_eq!(validated.palette, theme.palette);
}

#[test]
fn load_dir_skips_bad_files_and_tolerates_missing_dir() {
    let missing = Registry::load_dir(std::path::Path::new("/nonexistent/sunreactor/themes"));
    assert!(missing.diagnostics().is_empty());
    assert_eq!(missing.themes().len(), 40);

    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("Bad-Name.toml"), MINIMAL).unwrap();
    std::fs::write(dir.path().join("broken.toml"), "id = [").unwrap();
    std::fs::write(dir.path().join("notes.txt"), "ignored").unwrap();
    std::fs::write(dir.path().join("mine.toml"), MINIMAL).unwrap();
    let huge = format!("{MINIMAL}\n#{}\n", "x".repeat(70 * 1024));
    std::fs::write(dir.path().join("huge.toml"), huge).unwrap();

    let registry = Registry::load_dir(dir.path());
    assert_eq!(registry.themes().len(), 41);
    assert!(registry.get("mine").is_some());
    let stems: Vec<&str> = registry
        .diagnostics()
        .iter()
        .map(|d| d.stem.as_str())
        .collect();
    assert_eq!(stems.len(), 3, "{stems:?}");
    assert!(stems.contains(&"Bad-Name") && stems.contains(&"broken") && stems.contains(&"huge"));
    assert!(matches!(
        errors_for(&registry, "huge"),
        [ThemeError::TooLarge { .. }]
    ));
}

#[test]
fn fallback_resolution_never_fails_and_explains_why() {
    let registry = Registry::with_user_sources(vec![
        user(
            "nord",
            "id = \"nord\"\nname = \"Broken Nord\"\nfamily = \"terminal\"\n",
        ),
        user("mine", "id = \"mine\"\n[palette]\nbg = \"nope\"\n"),
        user("good", &MINIMAL.replace("\"mine\"", "\"good\"")),
    ]);

    // Valid user theme next to the broken ones loads normally.
    let (theme, warning) = registry.resolve_or_fallback("good");
    assert_eq!(theme.id, "good");
    assert!(warning.is_none());

    // Broken file shadowing a built-in: the built-in is used, file named.
    let (theme, warning) = registry.resolve_or_fallback("nord");
    assert_eq!(theme.id, "nord");
    assert_eq!(theme.source, ThemeSource::Builtin);
    let warning = warning.expect("broken nord.toml is reported");
    assert!(warning.contains("/virtual/themes/nord.toml"), "{warning}");
    assert!(
        warning.contains("using the built-in `nord` theme"),
        "{warning}"
    );

    // Broken user-only theme: default theme, file and reason named.
    let (theme, warning) = registry.resolve_or_fallback("mine");
    assert_eq!(theme.id, DEFAULT_THEME_ID);
    let warning = warning.expect("broken mine.toml is reported");
    assert!(warning.contains("/virtual/themes/mine.toml"), "{warning}");
    assert!(warning.contains("palette.bg"), "{warning}");

    // Typo: default theme with a suggestion.
    let (theme, warning) = registry.resolve_or_fallback("draculla");
    assert_eq!(theme.id, DEFAULT_THEME_ID);
    assert!(warning.unwrap().contains("did you mean `dracula`"));
}

#[test]
fn levenshtein_basics() {
    use super::registry::levenshtein_for_tests as lev;
    assert_eq!(lev("nord", "nord"), 0);
    assert_eq!(lev("nrod", "nord"), 2);
    assert_eq!(lev("", "abc"), 3);
    assert_eq!(lev("dracla", "dracula"), 1);
}
