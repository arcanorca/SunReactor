/*
 * The pixel family's design tokens: which hardware a theme looks like, and
 * which of its palette / shade colours play which role. Every colour here is
 * a theme token (palette entry or shade), never a literal, so any theme in the
 * pixel family - built-in or user-made - gets a coherent result and the
 * conformance check stays meaningful.
 *
 * `kind` is chosen by theme id only where the identity of the hardware really
 * differs (chrome, digits, dither, paper polarity); everything else, including
 * user themes, gets "neon": line chrome on the theme's own background.
 *
 *   dmg    Game Boy DMG LCD: light paper, 4-shade ink, LCD cells for digits
 *   c64    Commodore 64: PETSCII frames, reverse-video titles, blinking cursor
 *   mac    Macintosh 128K / System 1: 1-bit paper, striped title bars, dither
 *   casio  Casio digital watch: seven-segment digits with ghost segments
 *   amiga  Workbench: blue windows, white/black bevels, orange highlights
 *   dots   Nothing: dot matrix on black, red accent dot
 *   neon   everything else (cyberpunk, synthwave84, hackergreen, user themes)
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "Sprites.js" as Sprites

QtObject {
    id: root

    required property Theme theme

    readonly property string kind: {
        switch (theme.themeId) {
        case "handheld":
            return "dmg";
        case "commodore64":
            return "c64";
        case "classicmacintosh":
            return "mac";
        case "casiodigital":
            return "casio";
        case "amiga":
            return "amiga";
        case "nothing":
            return "dots";
        default:
            return "neon";
        }
    }

    // --- ramp ------------------------------------------------------------------
    readonly property var ramp: theme.shades
    readonly property color darkest: ramp[0]
    readonly property color lightest: ramp[ramp.length - 1]
    /*! Ramp step i from the dark end, clamped. */
    function shade(i) {
        return ramp[Math.max(0, Math.min(ramp.length - 1, i))];
    }
    /*! Ramp step i from the light end, clamped. */
    function shadeFromLight(i) {
        return shade(ramp.length - 1 - i);
    }

    /*! Panels on light paper with dark ink (the LCD and 1-bit looks). */
    readonly property bool lightPaper: kind === "dmg" || kind === "mac"
    readonly property bool oneBit: kind === "mac"

    // --- roles -------------------------------------------------------------------
    /*! Behind the panels: the popup background (FullRepresentation paints it). */
    readonly property color desk: theme.bg
    /*! Panel surface. */
    readonly property color paper: lightPaper ? lightest : (kind === "amiga" ? theme.secondaryAccent : theme.bg)
    /*! Primary text and lines. */
    readonly property color ink: lightPaper ? darkest : theme.fg
    /*! Secondary text. On 1-bit paper this is ink drawn stippled (dimText). */
    readonly property color dim: {
        switch (kind) {
        case "dmg":
            return shade(1);
        case "mac":
            return darkest;
        case "amiga":
            return theme.bg;
        default:
            return theme.textMuted;
        }
    }
    readonly property bool stippleDim: false
    /*! Unlit cells and segments, quiet rules. */
    readonly property color ghost: {
        switch (kind) {
        case "dmg":
            return shadeFromLight(1);
        case "mac":
            return darkest;
        case "amiga":
            return theme.bg;
        default:
            return theme.borderInactive;
        }
    }
    /*! Emphasis: values, the now marker, the selected cell. */
    readonly property color hot: {
        switch (kind) {
        case "dmg":
        case "mac":
            return darkest;
        case "amiga":
            return theme.borderActive;
        case "c64":
        case "dots":
        case "neon":
            return theme.accent;
        case "casio":
            return theme.borderActive;
        }
        return theme.accent;
    }
    /*! What the weather takes away, in the curve. */
    readonly property color cost: {
        switch (kind) {
        case "dmg":
            return shade(1);
        case "mac":
            return darkest;
        case "amiga":
            return theme.borderActive;
        case "c64":
        case "neon":
            return theme.secondaryAccent;
        case "casio":
            return theme.warning;
        default:
            return theme.textMuted;
        }
    }
    /*! Frame lines. */
    readonly property color frame: {
        switch (kind) {
        case "dmg":
        case "mac":
            return darkest;
        case "amiga":
            return theme.fg;
        case "c64":
            return theme.textMuted;
        default:
            return theme.borderInactive;
        }
    }
    /*! Printed captions (Casio bezel print, neon labels). */
    readonly property color caption: kind === "casio" ? theme.accent : (kind === "neon" ? theme.accent : dim)
    /*! Ink on a `hot` or `ink` block (reverse video). */
    readonly property color inverse: kind === "amiga" ? theme.bg : paper

    // --- shape ---------------------------------------------------------------------
    readonly property string chrome: {
        switch (kind) {
        case "dmg":
            return "textbox";
        case "c64":
            return "petscii";
        case "mac":
            return "window";
        case "casio":
            return "lcd";
        case "amiga":
            return "workbench";
        case "dots":
            return "card";
        default:
            return "chamfer";
        }
    }
    /*! Readout digits: LCD cells, seven segments, round dots or the bitmap font. */
    readonly property string digits: {
        switch (kind) {
        case "casio":
            return "seg";
        case "dots":
            return "dots";
        case "c64":
            return "bitmap";
        default:
            return "block";
        }
    }
    /*! Body text face: "small" (5x7 built-in) or "bitmap" (theme bitmap font). */
    readonly property string bodyFace: "small"
    /*! Titles: the bitmap font where the hardware had one 8x8 font. */
    readonly property string titleFace: kind === "c64" || kind === "dmg" ? "bitmap" : "small"
    readonly property bool capitalTitles: kind !== "mac" && kind !== "amiga"
    /*! Line height of one body row, art px. */
    readonly property int row: bodyFace === "bitmap" ? 10 : 10

    // Curve patterns.
    readonly property string areaPattern: {
        switch (kind) {
        case "mac":
            return "checker";
        case "c64":
            return "hlines";
        case "casio":
            return "vlines";
        case "dots":
            return "dots";
        case "amiga":
            return "checker";
        default:
            return "bayer4";
        }
    }
    readonly property color areaColor: {
        switch (kind) {
        case "dmg":
            return shadeFromLight(1);
        case "mac":
            return darkest;
        case "amiga":
            return theme.fg;
        case "c64":
            return theme.borderInactive;
        case "casio":
            return theme.borderInactive;
        case "dots":
            return theme.textMuted;
        default:
            return theme.borderInactive;
        }
    }

    // --- sprites ---------------------------------------------------------------------
    /*! Tone index -> colour for PixelSprite (see tools/generate_weather_icons.py):
        0 none, 1 highlight, 2 light, 3 mid, 4 shadow, 5 warm, 6 warm edge,
        7 water, 8 outline, 9 ray. */
    readonly property var spriteColors: {
        const none = "transparent";
        switch (kind) {
        case "dmg":
            return [none, lightest, shadeFromLight(1), shade(1), darkest, shadeFromLight(1), darkest,
                    shade(1), darkest, darkest];
        case "mac":
            return [none, lightest, lightest, darkest, darkest, lightest, darkest, darkest, darkest, darkest];
        case "amiga":
            return [none, theme.fg, theme.fg, theme.textMuted, theme.bg, theme.borderActive, theme.accent,
                    theme.fg, theme.bg, theme.borderActive];
        case "c64":
            return [none, theme.accent, theme.fg, theme.textMuted, theme.borderInactive, theme.warning,
                    theme.secondaryAccent, theme.secondaryAccent, none, theme.fg];
        case "casio":
            return [none, theme.fg, theme.fg, theme.secondaryAccent, theme.textMuted, theme.borderActive,
                    theme.success, theme.accent, none, theme.borderActive];
        case "dots":
            return [none, theme.fg, theme.success, theme.secondaryAccent, theme.textMuted, theme.fg,
                    theme.accent, theme.secondaryAccent, none, theme.accent];
        default:
            return [none, theme.fg, theme.fg, theme.textMuted, theme.borderInactive, theme.warning,
                    theme.accent, theme.secondaryAccent, none, theme.warning];
        }
    }
    /*! Same idea for the 8x8 symbols drawn on a reverse-video (hot/ink) block. */
    readonly property var symbolColors: {
        const c = spriteColors.slice();
        return c;
    }
    /*! The handheld's four-shade ramp (Sprites.isMonoRamp): weather art in those shades only. */
    readonly property bool dmgRamp: Sprites.isMonoRamp(theme)
    /*! Weather is the 16 px sprite on the handheld whatever the icon style: the vector
        glyphs blend colours between their roles, so they cannot be shade-only. Other
        pixel themes keep their own weather icon style. */
    readonly property bool spriteWeather: dmgRamp || theme.glyphStyle === "pixel"
    /*! PixelSprite colours for the weather frame `frame` (an index into Sprites.WEATHER). */
    function weatherColors(frame) {
        return dmgRamp ? Sprites.monoWeatherColors(frame, ramp) : spriteColors;
    }
}
