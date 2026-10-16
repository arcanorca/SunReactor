/*
 * Typed design tokens for one resolved theme. Modules and skins read colours,
 * fonts, metrics and the pixel grid from here and from nowhere else.
 *
 * `spec` is a normalised theme map from the plugin (see plugin/themenormalize.h:
 * id, name, family, glyphStyle, legacy, palette{...}, widget{...}); `overrides`
 * holds the per-instance tweaks the ThemeEngine applies on top. With an empty
 * spec (offline, or a daemon that sends no theme) every token falls back to the
 * Plasma colour scheme and the vector renderer, so the widget still looks like
 * a native Plasma popup.
 *
 * The Theme is an Item only because the pixel grid needs a window to know the
 * device pixel ratio; it has no size and draws nothing.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../modules/Modules.js" as Modules
import "ScaleHelper.js" as ScaleHelper

Item {
    id: root

    /*! Normalised theme map (client.theme, client.pinnedTheme or a preview). */
    property var spec: ({})
    /*! { accent: "#rrggbb", density: "compact"|"comfortable", renderer: ...,
          pixelScale: 0..6, modules: [ids], weatherIconStyle: ... } - empty values
          mean "keep". */
    property var overrides: ({})
    /*! "follow", "pinned" or "plasma" (no theme at all); set by the engine. */
    property string source: "plasma"

    visible: false
    width: 0
    height: 0

    // --- identity ------------------------------------------------------------

    readonly property var paletteMap: spec && spec.palette ? spec.palette : null
    readonly property var widget: spec && spec.widget ? spec.widget : ({})
    /*! False when there is no daemon theme and Plasma colours are in use. */
    readonly property bool available: paletteMap !== null
    readonly property string themeId: available ? (spec.id || "") : ""
    readonly property string name: available ? (spec.name || themeId) : ""
    readonly property string family: available ? spec.family : "instrument"
    /*! The TUI's own glyph style, "rounded" or "pixel" (box corners). */
    readonly property string tuiGlyphStyle: available ? (spec.glyphStyle || "rounded") : "rounded"
    readonly property bool legacy: available && spec.legacy === true

    readonly property string renderer: {
        const wanted = overrides && overrides.renderer ? overrides.renderer : "";
        if (["pixel", "vector", "terminal"].indexOf(wanted) >= 0) {
            return wanted;
        }
        return available ? (widget.renderer || "vector") : "vector";
    }
    readonly property bool isPixel: renderer === "pixel"
    readonly property bool isTerminal: renderer === "terminal"
    readonly property bool isVector: renderer === "vector"

    // --- palette ---------------------------------------------------------------

    function pick(key, fallback) {
        return available && paletteMap[key] ? paletteMap[key] : fallback;
    }

    readonly property color bg: pick("bg", Kirigami.Theme.backgroundColor)
    readonly property color fg: pick("fg", Kirigami.Theme.textColor)
    readonly property color accent: {
        const wanted = overrides && overrides.accent ? String(overrides.accent) : "";
        if (/^#[0-9a-fA-F]{6}$/.test(wanted)) {
            return wanted;
        }
        return pick("accent", Kirigami.Theme.highlightColor);
    }
    readonly property color secondaryAccent: pick("secondaryAccent", accent)
    readonly property color borderActive: pick("borderActive", Kirigami.Theme.highlightColor)
    readonly property color borderInactive: pick("borderInactive", mix(bg, fg, 0.2))
    readonly property color textMuted: pick("textMuted", Kirigami.Theme.disabledTextColor)
    readonly property color success: pick("success", Kirigami.Theme.positiveTextColor)
    readonly property color warning: pick("warning", Kirigami.Theme.neutralTextColor)
    readonly property color error: pick("error", Kirigami.Theme.negativeTextColor)

    /*! The pixel ramp, dark to light. From the theme when it sends 2..8
        colours; otherwise derived from the palette by luminance, so every shade
        is a palette colour and the conformance check stays meaningful. */
    readonly property var shades: {
        const given = widget.shades;
        if (available && given && given.length >= 2) {
            return given.map(c => Qt.color(c));
        }
        const pool = [bg, borderInactive, textMuted, fg];
        const unique = [];
        for (const c of pool) {
            if (!unique.some(u => Qt.colorEqual(u, c))) {
                unique.push(c);
            }
        }
        unique.sort((a, b) => luminance(a) - luminance(b));
        return unique.length >= 2 ? unique : [bg, fg];
    }
    /*! Every colour a pixel surface may contain (palette + shades). */
    readonly property var pixelColors: {
        const all = [bg, fg, accent, secondaryAccent, borderActive, borderInactive, textMuted, success,
                     warning, error].concat(shades);
        const unique = [];
        for (const c of all) {
            if (!unique.some(u => Qt.colorEqual(u, c))) {
                unique.push(c);
            }
        }
        return unique;
    }

    // Surfaces for the vector renderer (pixel themes use shades instead).
    readonly property color surface: available ? bg : "transparent"
    readonly property color raised: available ? mix(bg, fg, 0.06) : Qt.rgba(fg.r, fg.g, fg.b, 0.04)
    readonly property color line: available ? borderInactive : Qt.rgba(fg.r, fg.g, fg.b, 0.12)

    // --- widget tokens -----------------------------------------------------------

    readonly property string density: {
        const wanted = overrides && overrides.density ? overrides.density : "";
        if (wanted === "compact" || wanted === "comfortable") {
            return wanted;
        }
        return available ? (widget.density || "comfortable") : "comfortable";
    }
    readonly property bool compact: density === "compact"
    readonly property string border: available ? (widget.border || "line") : "none"
    /*! Corner radius in logical px; always 0 for pixel and terminal renderers. */
    readonly property real radius: isVector ? (available ? (widget.radius !== undefined ? widget.radius : 4)
                                                         : Kirigami.Units.cornerRadius)
                                            : 0
    readonly property string dither: available ? (widget.dither || "none") : "none"
    readonly property real glow: available ? (widget.glow || 0) : 0
    readonly property int pixelScale: {
        const wanted = overrides && overrides.pixelScale ? overrides.pixelScale : 0;
        if (wanted > 0) {
            return Math.min(6, wanted);
        }
        const base = available && widget.pixelScale > 0 ? widget.pixelScale : 2;
        return Math.max(1, Math.min(6, Math.round(base * scaleFactor)));
    }

    // --- scaling (UI scale / auto sizing) -------------------------------------------

    /*! uiScale from overrides (0 = automatic based on screen resolution and dimension). */
    readonly property int uiScale: {
        const wanted = overrides && overrides.uiScale !== undefined ? Number(overrides.uiScale) : 0;
        return isNaN(wanted) ? 0 : wanted;
    }

    /*! Automatic scale factor computed from screen resolution and physical dimensions. */
    readonly property real autoScaleFactor: ScaleHelper.calculateAutoScale(Screen)

    /*! Resolved effective scale factor (e.g. 1.0, 1.15, 1.25, 1.5). */
    readonly property real scaleFactor: ScaleHelper.resolveScale(uiScale, Screen)

    /*! Scaled grid unit, anchored to Kirigami.Units.gridUnit. */
    readonly property real gridUnit: Math.round(Kirigami.Units.gridUnit * scaleFactor)

    /*! Scaled font pixel sizes, anchored to gridUnit so they scale reliably with UI scale. */
    readonly property int fontSizeBody: Math.max(11, Math.round(gridUnit * 0.72))
    readonly property int fontSizeSmall: Math.max(9, Math.round(gridUnit * 0.58))
    readonly property int fontSizeHeader: Math.max(13, Math.round(gridUnit * 0.88))

    // --- weather icons ---------------------------------------------------------------

    /*! The weather icon style the user chose: "auto" (the theme's own icon, the
        default), or an explicit "modern" (full colour), "line", "solid" or "pixel".
        Any other value counts as "auto". ThemedWeatherIcon reads it. */
    readonly property string weatherIconStyle: {
        const wanted = overrides && overrides.weatherIconStyle ? overrides.weatherIconStyle : "";
        return ["modern", "line", "solid", "pixel"].indexOf(wanted) >= 0 ? wanted : "auto";
    }
    /*! Weather icon style for WeatherGlyph: "line" (outline), "solid" (filled) or
        "pixel" (the legacy sprites). An explicit line, solid or pixel style wins;
        "auto" and "modern" follow the renderer: pixel themes are filled, the rest
        outlined. */
    readonly property string glyphStyle: {
        if (["line", "solid", "pixel"].indexOf(weatherIconStyle) >= 0) {
            return weatherIconStyle;
        }
        return isPixel ? "solid" : "line";
    }
    /*! WeatherGlyph colour roles on this theme's own background. */
    readonly property var glyphPalette: glyphColours(bg, fg)
    /*! Popup modules in order: the `modules` override (a full order chosen
        in settings) or the theme's list, minus `hidden` ones; unknown and
        repeated ids are dropped. May be empty when everything is hidden. */
    readonly property var modules: {
        const wanted = overrides && overrides.modules && overrides.modules.length ? overrides.modules : null;
        const base = wanted
            ? wanted
            : (available && widget.modules && widget.modules.length ? widget.modules : Object.keys(Modules.FILES));
        const hidden = overrides && overrides.hidden ? overrides.hidden : [];
        const out = [];
        for (const id of base) {
            const key = String(id);
            if (Modules.file(key).length > 0 && hidden.indexOf(key) < 0 && out.indexOf(key) < 0) {
                out.push(key);
            }
        }
        return out;
    }

    // --- metrics (logical px) ------------------------------------------------------

    readonly property real spacing: Math.round((compact ? Kirigami.Units.smallSpacing : Kirigami.Units.largeSpacing) * scaleFactor)
    readonly property real padding: Math.round((compact ? Kirigami.Units.smallSpacing * 2 : Kirigami.Units.largeSpacing * 1.5) * scaleFactor)
    readonly property real sectionSpacing: compact ? Math.round(Kirigami.Units.smallSpacing * 2 * scaleFactor) : gridUnit
    /*! One device pixel, for hairlines that must stay crisp in vector themes. */
    readonly property real hairline: 1 / grid.devicePixelRatio
    readonly property real lineWidth: Math.max(1, Math.round(grid.devicePixelRatio)) / grid.devicePixelRatio

    // --- fonts ---------------------------------------------------------------------

    readonly property string fontRole: available ? (widget.font || "sans") : "sans"

    FontLoader { id: pressStart; source: Qt.resolvedUrl("../../fonts/PressStart2P.ttf") }
    FontLoader { id: unscii; source: Qt.resolvedUrl("../../fonts/unscii-16.ttf") }
    FontLoader { id: plexSans; source: Qt.resolvedUrl("../../fonts/IBMPlexSans-Regular.ttf") }
    FontLoader { id: plexMono; source: Qt.resolvedUrl("../../fonts/IBMPlexMono-Regular.ttf") }
    FontLoader { id: orbitron; source: Qt.resolvedUrl("../../fonts/Orbitron-Variable.ttf") }

    /*! Family for BitmapText, "" when the theme has no bitmap font or it
        failed to load (BitmapText then falls back to vector mono). */
    readonly property string bitmapFont: {
        if (fontRole === "bitmap8" || (isPixel && fontRole !== "bitmap_vga")) {
            return pressStart.status === FontLoader.Ready ? pressStart.font.family : "";
        }
        if (fontRole === "bitmap_vga") {
            return unscii.status === FontLoader.Ready ? unscii.font.family : "";
        }
        return "";
    }
    /*! The bitmap font's native grid: glyphs are crisp at whole multiples only. */
    readonly property int bitmapPixelSize: fontRole === "bitmap_vga" ? 16 : 8

    readonly property string monoFont: plexMono.status === FontLoader.Ready ? plexMono.font.family : "monospace"
    readonly property string sansFont: plexSans.status === FontLoader.Ready ? plexSans.font.family : ""
    /*! Body text in vector form. Bitmap themes get mono here: a bitmap font at a
        non-native size blurs, so vector text never uses one. */
    readonly property string bodyFont: {
        if (!available) {
            return "";
        }
        switch (fontRole) {
        case "sans":
        case "display":
            return sansFont;
        default:
            return monoFont;
        }
    }
    readonly property string digitsFont: {
        if (!available) {
            return "";
        }
        if (fontRole === "display") {
            return orbitron.status === FontLoader.Ready ? orbitron.font.family : monoFont;
        }
        return monoFont;
    }

    // --- pixel grid ----------------------------------------------------------------

    /*! Device pixel ratio, art-pixel size (deviceScale/unit), snap helpers. */
    readonly property PixelGrid grid: pixelGrid
    /*! The logical width the terminal skins' character grid spans: the popup's
        page column less the scrollbar's gutter (FullRepresentation binds it; 0
        until the popup is laid out). It does not change when a page starts to
        scroll, so the cell, the row heights and so the page height stay put. */
    property real gridContentWidth: 0

    PixelGrid {
        id: pixelGrid
        preferredScale: root.pixelScale
    }

    // --- helpers -------------------------------------------------------------------

    function luminance(c) {
        const lin = v => v <= 0.03928 ? v / 12.92 : Math.pow((v + 0.055) / 1.055, 2.4);
        return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
    }

    function mix(a, b, t) {
        return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1);
    }

    function alpha(c, a) {
        return Qt.rgba(c.r, c.g, c.b, a);
    }

    function hexOf(c) {
        const k = Qt.color(c);
        const part = v => ("0" + Math.round(v * 255).toString(16)).slice(-2);
        return "#" + part(k.r) + part(k.g) + part(k.b);
    }

    /*! WeatherGlyph roles as "#rrggbb" strings for a surface of `background` with
        `ink` text. WeatherGlyph lifts any role below 3:1 against `background`
        itself, so the theme's tokens serve panels and pixel surfaces alike. */
    function glyphColours(background, ink) {
        return {
            bg: hexOf(background),
            fg: hexOf(ink),
            text_muted: hexOf(textMuted),
            accent: hexOf(accent),
            secondary_accent: hexOf(secondaryAccent),
            warning: hexOf(warning)
        };
    }

    /*! The palette colour (fg or bg) that reads best on `background`. */
    function onColor(background) {
        const lb = luminance(background);
        const contrast = c => (Math.max(lb, luminance(c)) + 0.05) / (Math.min(lb, luminance(c)) + 0.05);
        return contrast(fg) >= contrast(bg) ? fg : bg;
    }

    /*! Telemetry/state colour for a SunReactorClient.Mode value. */
    function stateColor(mode) {
        switch (mode) {
        case SunReactorClient.Offline:
            return error;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return warning;
        default:
            return success;
        }
    }
}
