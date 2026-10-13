/*
 * Theme engine unit tests (qmltestrunner, offscreen): token derivation, family
 * fallbacks, overrides, font roles, the Plasma fallback without a theme, the
 * pixel grid arithmetic, and the follow/pin resolution against an offline
 * client. No daemon, no socket.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import org.sunreactor.plasma

import "../package/contents/ui/engine"

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
    }
}
