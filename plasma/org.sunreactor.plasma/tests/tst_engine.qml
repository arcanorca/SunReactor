/*
 * Theme engine unit tests (qmltestrunner, offscreen): token derivation, family
 * fallbacks, overrides, font roles, the Plasma fallback without a theme, the
 * pixel grid arithmetic, and the follow/pin resolution against an offline
 * client. No daemon, no socket.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../package/contents/ui/engine"
import "../package/contents/ui/engine/ScaleHelper.js" as ScaleHelper

Item {
    id: top

    width: 200
    height: 200

    // Normalised maps as the plugin produces them (plugin/themenormalize.h).
    readonly property var handheld: ({
        id: "handheld", name: "Handheld", family: "pixel", glyphStyle: "pixel", legacy: false,
        palette: { bg: "#141c12", fg: "#9bbc0f", accent: "#8b1d42", secondaryAccent: "#8bac0f",
                   borderActive: "#9bbc0f", borderInactive: "#283824", textMuted: "#567648",
                   success: "#8bac0f", warning: "#c8a646", error: "#9a2257" },
        widget: { renderer: "pixel", font: "bitmap8", pixelScale: 0, density: "compact", border: "line",
                  radius: 0, dither: "bayer4", glow: 0, shades: ["#141c12", "#306230", "#8bac0f", "#9bbc0f"],
                  modules: ["telemetry", "curve", "displays"] },
    })
    readonly property var c64: ({
        id: "commodore64", name: "Commodore 64", family: "pixel", glyphStyle: "pixel", legacy: false,
        palette: { bg: "#101225", fg: "#a7b7ff", accent: "#ffffff", secondaryAccent: "#7fe5e5",
                   borderActive: "#ffffff", borderInactive: "#3b3f78", textMuted: "#6672a8",
                   success: "#ffffff", warning: "#ffffff", error: "#ff0000" },
        widget: { renderer: "pixel", font: "bitmap_vga", pixelScale: 3, density: "compact", border: "double",
                  radius: 0, dither: "bayer2", glow: 0, shades: [],
                  modules: ["telemetry", "curve", "displays", "automation", "weather", "forecast", "suntimes"] },
    })
    readonly property var braun: ({
        id: "braun", name: "Braun", family: "instrument", glyphStyle: "rounded", legacy: false,
        palette: { bg: "#161616", fg: "#f2f2f0", accent: "#ff5500", secondaryAccent: "#f0b01e",
                   borderActive: "#ff5500", borderInactive: "#404040", textMuted: "#8a8781",
                   success: "#248a3d", warning: "#d97706", error: "#dc2626" },
        widget: { renderer: "vector", font: "sans", pixelScale: 0, density: "comfortable", border: "line",
                  radius: 4, dither: "none", glow: 0, shades: [],
                  modules: ["telemetry", "curve", "displays", "automation", "weather", "forecast", "suntimes"] },
    })

    Theme { id: pixelTheme; spec: top.handheld }
    Theme { id: vgaTheme; spec: top.c64 }
    Theme { id: vectorTheme; spec: top.braun }
    Theme { id: plasmaTheme }
    Theme { id: overridden; spec: top.braun }

    SunReactorClient {
        id: offline
        socketPath: "/nonexistent/sunreactor-test.sock"
    }

    ThemeEngine {
        id: engine
        client: offline
    }

    TestCase {
        name: "ThemeEngine"
        when: windowShown

        function test_pixelTokens() {
            compare(pixelTheme.available, true);
            compare(pixelTheme.renderer, "pixel");
            verify(pixelTheme.isPixel);
            compare(pixelTheme.radius, 0, "pixel renderer never rounds");
            compare(pixelTheme.shades.length, 4);
            verify(Qt.colorEqual(pixelTheme.shades[1], "#306230"), "theme shades are used as sent");
            compare(pixelTheme.modules, ["telemetry", "curve", "displays"]);
            compare(pixelTheme.bitmapPixelSize, 8);
            tryVerify(() => pixelTheme.bitmapFont.length > 0, 3000, "Press Start 2P loads from the package");
            compare(pixelTheme.bitmapFont, "Press Start 2P");
        }

        function test_derivedShadesAreSortedPaletteColours() {
            const shades = vgaTheme.shades;
            compare(shades.length, 4);
            for (let i = 1; i < shades.length; ++i) {
                verify(vgaTheme.luminance(shades[i - 1]) <= vgaTheme.luminance(shades[i]), "dark to light");
            }
            verify(Qt.colorEqual(shades[0], "#101225"), "darkest is bg");
            verify(Qt.colorEqual(shades[3], "#a7b7ff"), "lightest is fg");
            // Every derived shade is a palette colour, so conformance stays exact.
            for (const s of shades) {
                verify(vgaTheme.pixelColors.some(c => Qt.colorEqual(c, s)));
            }
        }

        function test_vgaFontAndScale() {
            compare(vgaTheme.bitmapPixelSize, 16);
            tryVerify(() => vgaTheme.bitmapFont === "unscii", 3000);
            compare(vgaTheme.pixelScale, 3);
            compare(vgaTheme.grid.preferredScale, 3);
        }

        function test_vectorTokens() {
            compare(vectorTheme.renderer, "vector");
            compare(vectorTheme.radius, 4);
            compare(vectorTheme.bitmapFont, "", "vector themes have no bitmap font");
            compare(vectorTheme.compact, false);
            tryVerify(() => vectorTheme.bodyFont === "IBM Plex Sans", 3000);
            tryVerify(() => vectorTheme.digitsFont === "IBM Plex Mono", 3000);
        }

        function test_plasmaFallbackWithoutTheme() {
            compare(plasmaTheme.available, false);
            compare(plasmaTheme.renderer, "vector");
            compare(plasmaTheme.bodyFont, "", "Plasma's own font");
            compare(plasmaTheme.modules.length, 7);
            verify(plasmaTheme.fg.a > 0);
        }

        function test_overrides() {
            overridden.overrides = { accent: "#00ff00", density: "compact", renderer: "terminal", pixelScale: 9 };
            verify(Qt.colorEqual(overridden.accent, "#00ff00"));
            compare(overridden.density, "compact");
            compare(overridden.renderer, "terminal");
            compare(overridden.radius, 0);
            compare(overridden.pixelScale, 6, "clamped");
            overridden.overrides = { accent: "green-ish", density: "huge", renderer: "hologram" };
            verify(Qt.colorEqual(overridden.accent, "#ff5500"), "invalid accent keeps the theme's");
            compare(overridden.density, "comfortable");
            compare(overridden.renderer, "vector");
            overridden.overrides = { modules: ["displays"] };
            compare(overridden.modules, ["displays"]);
        }

        function test_weatherGlyphStyle() {
            compare(pixelTheme.glyphStyle, "solid", "pixel themes are filled");
            compare(vectorTheme.glyphStyle, "line", "vector themes are outlined");
            compare(plasmaTheme.glyphStyle, "line");
            compare(pixelTheme.tuiGlyphStyle, "pixel", "the TUI's corner style keeps its own name");
            compare(vectorTheme.tuiGlyphStyle, "rounded");
            overridden.overrides = { weatherIconStyle: "pixel" };
            compare(overridden.glyphStyle, "pixel", "legacy sprites when asked for");
            overridden.overrides = { weatherIconStyle: "solid" };
            compare(overridden.glyphStyle, "solid");
            overridden.overrides = { weatherIconStyle: "auto" };
            compare(overridden.glyphStyle, "line", "auto follows the renderer");
            overridden.overrides = { weatherIconStyle: "neon" };
            compare(overridden.glyphStyle, "line", "an unknown style keeps auto");
            overridden.overrides = {};
        }

        function test_weatherIconStyle() {
            compare(vectorTheme.weatherIconStyle, "auto", "the theme's own icon by default");
            overridden.overrides = { weatherIconStyle: "modern" };
            compare(overridden.weatherIconStyle, "modern");
            compare(overridden.glyphStyle, "line", "modern leaves the glyph style to the renderer");
            overridden.overrides = { weatherIconStyle: "pixel" };
            compare(overridden.weatherIconStyle, "pixel");
            overridden.overrides = { weatherIconStyle: "neon" };
            compare(overridden.weatherIconStyle, "auto", "an unknown style is auto");
            overridden.overrides = {};
        }

        function test_glyphPalette() {
            const p = vectorTheme.glyphPalette;
            compare(p.bg, "#161616");
            compare(p.fg, "#f2f2f0");
            compare(p.accent, "#ff5500");
            compare(p.secondary_accent, "#f0b01e");
            compare(p.warning, "#d97706");
            compare(p.text_muted, "#8a8781");
            compare(vectorTheme.glyphColours("#ffffff", "#000000").bg, "#ffffff", "for another surface");
        }

        function test_gridArithmetic() {
            const g = pixelTheme.grid;
            verify(g.devicePixelRatio > 0);
            // k = max(2, round(2 * dpr)) device px per art px; unit = k / dpr.
            compare(g.deviceScale, Math.max(2, Math.round(2 * g.devicePixelRatio)));
            fuzzyCompare(g.unit, g.deviceScale / g.devicePixelRatio, 1e-9);
            compare(g.cells(g.unit * 10 + 0.5), 10);
            fuzzyCompare(g.snap(1.3) * g.devicePixelRatio, Math.round(1.3 * g.devicePixelRatio), 1e-9);
        }

        function test_engineFollowsAndExplainsPins() {
            compare(engine.theme.source, "plasma", "offline: Plasma colours");
            compare(engine.notice, "");
            engine.mode = "pinned";
            engine.pinnedId = "nord";
            compare(offline.pinnedThemeId, "nord", "the pin reaches the client");
            verify(engine.notice.length > 0, "a pin that is not in effect says why");
            compare(engine.theme.source, "plasma", "and the engine keeps following meanwhile");
            engine.mode = "follow";
            compare(offline.pinnedThemeId, "");
            compare(engine.notice, "");
        }

        function test_scaleHelperAutoScale() {
            compare(ScaleHelper.calculateAutoScale(null), 1.0);
            compare(ScaleHelper.calculateAutoScale({ width: 0, height: 0 }), 1.15, "1080p fallback");

            // 1080p desktop with 96 DPI
            const fhd = { width: 1920, height: 1080, devicePixelRatio: 1.0, desktopAvailableHeight: 1050, pixelDensity: 96 / 25.4 };
            compare(ScaleHelper.calculateAutoScale(fhd), 1.15);

            // 14-inch FHD laptop at DPR 1.0 (157 physical DPI)
            const laptop100 = { width: 1920, height: 1080, devicePixelRatio: 1.0, desktopAvailableHeight: 1050, pixelDensity: 157.2 / 25.4 };
            compare(ScaleHelper.calculateAutoScale(laptop100), 1.25, "14-inch 1080p laptop at DPR 1.0");

            // 14-inch FHD laptop at DPR 1.25 (scaled by Wayland/Plasma: 1536x864 logical, 157 physical DPI)
            const laptop125 = { width: 1536, height: 864, devicePixelRatio: 1.25, desktopAvailableHeight: 840, pixelDensity: (157.2 / 1.25) / 25.4 };
            compare(ScaleHelper.calculateAutoScale(laptop125), 1.25, "14-inch 1080p laptop at DPR 1.25");

            // 1440p desktop (2560x1440)
            const qhd = { width: 2560, height: 1440, devicePixelRatio: 1.0, desktopAvailableHeight: 1400, pixelDensity: 0 };
            compare(ScaleHelper.calculateAutoScale(qhd), 1.25);

            // 4K UHD desktop (3840x2160, DPR 1.0)
            const uhd = { width: 3840, height: 2160, devicePixelRatio: 1.0, desktopAvailableHeight: 2100, pixelDensity: 0 };
            compare(ScaleHelper.calculateAutoScale(uhd), 1.50);

            // Low resolution screen (e.g. 1280x720)
            const lowRes = { width: 1280, height: 720, devicePixelRatio: 1.0, desktopAvailableHeight: 700, pixelDensity: 0 };
            compare(ScaleHelper.calculateAutoScale(lowRes), 1.0);

            // Height-constrained display
            const smallH = { width: 1920, height: 600, devicePixelRatio: 1.0, desktopAvailableHeight: 500, pixelDensity: 0 };
            verify(ScaleHelper.calculateAutoScale(smallH) <= 1.0, "constrained height avoids overflow");

            // resolveScale explicit vs auto
            compare(ScaleHelper.resolveScale(150, null), 1.5);
            compare(ScaleHelper.resolveScale(50, null), 0.5);
            compare(ScaleHelper.resolveScale(300, null), 3.0);
            compare(ScaleHelper.resolveScale(0, null), 1.0);
        }

        function test_themeUiScale() {
            verify(vectorTheme.scaleFactor >= 0.5 && vectorTheme.scaleFactor <= 3.0);
            compare(vectorTheme.gridUnit, Math.round(Kirigami.Units.gridUnit * vectorTheme.scaleFactor));
            const expectedSpacing = Math.round((vectorTheme.compact ? Kirigami.Units.smallSpacing : Kirigami.Units.largeSpacing) * vectorTheme.scaleFactor);
            compare(vectorTheme.spacing, expectedSpacing);

            // Font token hierarchy tests
            compare(vectorTheme.fontSizeBody, Math.max(11, Math.round(vectorTheme.gridUnit * 0.72)));
            compare(vectorTheme.fontSizeSmall, Math.max(9, Math.round(vectorTheme.gridUnit * 0.58)));
            compare(vectorTheme.fontSizeHeader, Math.max(13, Math.round(vectorTheme.gridUnit * 0.88)));
            verify(vectorTheme.fontSizeBody > vectorTheme.fontSizeSmall, "body text is strictly larger than small text");

            overridden.spec = top.braun;
            overridden.overrides = { uiScale: 150 };
            compare(overridden.scaleFactor, 1.5);
            compare(overridden.gridUnit, Math.round(Kirigami.Units.gridUnit * 1.5));
            const expectedOverriddenSpacing = Math.round((overridden.compact ? Kirigami.Units.smallSpacing : Kirigami.Units.largeSpacing) * 1.5);
            compare(overridden.spacing, expectedOverriddenSpacing);

            // Proportional pixelScale scaling across UI scale factors
            overridden.spec = top.handheld; // base 0 -> default 2
            overridden.overrides = { uiScale: 100 };
            compare(overridden.pixelScale, 2, "pixelScale at 100% is base 2");
            overridden.overrides = { uiScale: 150 };
            compare(overridden.pixelScale, 3, "pixelScale at 150% scales to 3");
            overridden.overrides = { uiScale: 200 };
            compare(overridden.pixelScale, 4, "pixelScale at 200% scales to 4");

            // C64 with base 3
            overridden.spec = top.c64;
            overridden.overrides = { uiScale: 100 };
            compare(overridden.pixelScale, 3, "C64 pixelScale at 100% is 3");
            overridden.overrides = { uiScale: 200 };
            compare(overridden.pixelScale, 6, "C64 pixelScale at 200% scales to 6");

            // Clamping tests
            overridden.overrides = { uiScale: 20 };
            compare(overridden.scaleFactor, 0.5, "clamped to minimum 0.5");
            overridden.overrides = { uiScale: 500 };
            compare(overridden.scaleFactor, 3.0, "clamped to maximum 3.0");

            overridden.spec = top.braun;
            overridden.overrides = {};
        }

        function test_engineUiScaleOverride() {
            engine.uiScaleOverride = 125;
            compare(engine.theme.scaleFactor, 1.25);
            engine.uiScaleOverride = 0;
            verify(engine.theme.scaleFactor > 0);
        }
    }
}
