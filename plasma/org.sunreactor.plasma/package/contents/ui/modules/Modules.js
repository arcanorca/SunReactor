.pragma library

/*
 * Module registry: the ids a theme's `widget.modules` list may contain
 * (DESIGN.md section 2) and the file that implements each. FullRepresentation
 * loads them in the theme's order.
 *
 * Module contract - every file listed here:
 *   required property Theme theme              resolved tokens (engine/Theme.qml)
 *   required property SunReactorClient client  daemon data and commands
 *   required property ModuleSettings settings  plasmoid settings it may need
 *   readonly property bool available           false hides the module (no data,
 *                                              offline, unsupported daemon)
 *   implicitHeight / implicitWidth             from its content; width is given
 * Modules read colours, fonts and metrics from `theme` only, never from
 * Plasmoid.configuration or Kirigami directly, so a skin can replace one file
 * without touching the others.
 */

var FILES = {
    telemetry: "Telemetry.qml",
    curve: "LightCycle.qml",
    displays: "Displays.qml",
    automation: "Automation.qml",
    weather: "Weather.qml",
    forecast: "Forecast.qml",
    suntimes: "SunTimes.qml"
};

/*
 * Skins replace one module for a renderer or a single theme. Keys are
 * "id:<theme id>" (flagship, wins) or the renderer name ("pixel", "terminal",
 * "vector"); values map a module id to a path relative to ui/ (normally
 * skins/<name>/<Module>.qml). A skin file follows the same module contract.
 * Each skin author edits only the entries for their own key.
 */
// Instrument family (skins/instrument): one set of files for every vector
// theme; flagship identities (VFD, Nixie, scope, ThinkPad, Braun) are chosen
// inside from the theme id and tokens (InstrumentStyle.flavour), so switching
// between instrument themes needs no reload.
var INSTRUMENT = {
    telemetry: "skins/instrument/Telemetry.qml",
    curve: "skins/instrument/LightCycle.qml",
    displays: "skins/instrument/Displays.qml",
    automation: "skins/instrument/Automation.qml",
    weather: "skins/instrument/Weather.qml",
    forecast: "skins/instrument/Forecast.qml",
    suntimes: "skins/instrument/SunTimes.qml"
};

var SKINS = {
    "pixel": {
        telemetry: "skins/pixel/Telemetry.qml",
        curve: "skins/pixel/LightCycle.qml",
        displays: "skins/pixel/Displays.qml",
        automation: "skins/pixel/Automation.qml",
        weather: "skins/pixel/Weather.qml",
        forecast: "skins/pixel/Forecast.qml",
        suntimes: "skins/pixel/SunTimes.qml"
    },
    "terminal": {
        telemetry: "skins/terminal/Telemetry.qml",
        curve: "skins/terminal/LightCycle.qml",
        displays: "skins/terminal/Displays.qml",
        automation: "skins/terminal/Automation.qml",
        weather: "skins/terminal/Weather.qml",
        forecast: "skins/terminal/Forecast.qml",
        suntimes: "skins/terminal/SunTimes.qml"
    },
    "vector": INSTRUMENT,
    "id:braun": INSTRUMENT,
    "id:vfd_hifi": INSTRUMENT,
    "id:nixie": INSTRUMENT,
    "id:oscilloscope": INSTRUMENT,
    "id:thinkpad": INSTRUMENT,
    "id:casiodigital": {
        telemetry: "skins/casiodigital/Telemetry.qml",
        curve: "skins/casiodigital/LightCycle.qml",
        displays: "skins/casiodigital/Displays.qml",
        automation: "skins/casiodigital/Automation.qml",
        weather: "skins/casiodigital/Weather.qml",
        forecast: "skins/casiodigital/Forecast.qml",
        suntimes: "skins/casiodigital/SunTimes.qml"
    },
    "id:hackergreen": {
        telemetry: "skins/hackergreen/Telemetry.qml",
        curve: "skins/hackergreen/LightCycle.qml",
        displays: "skins/hackergreen/Displays.qml",
        automation: "skins/hackergreen/Automation.qml",
        weather: "skins/hackergreen/Weather.qml",
        forecast: "skins/hackergreen/Forecast.qml",
        suntimes: "skins/hackergreen/SunTimes.qml"
    },
    "id:synthwave84": {
        telemetry: "skins/synthwave84/Telemetry.qml",
        curve: "skins/synthwave84/LightCycle.qml",
        displays: "skins/synthwave84/Displays.qml",
        automation: "skins/synthwave84/Automation.qml",
        weather: "skins/synthwave84/Weather.qml",
        forecast: "skins/synthwave84/Forecast.qml",
        suntimes: "skins/synthwave84/SunTimes.qml"
    },
    "id:cyberpunk": {
        telemetry: "skins/cyberpunk/Telemetry.qml",
        curve: "skins/cyberpunk/LightCycle.qml",
        displays: "skins/cyberpunk/Displays.qml",
        automation: "skins/cyberpunk/Automation.qml",
        weather: "skins/cyberpunk/Weather.qml",
        forecast: "skins/cyberpunk/Forecast.qml",
        suntimes: "skins/cyberpunk/SunTimes.qml"
    },
    "id:winxp": {
        telemetry: "skins/winxp/Telemetry.qml",
        curve: "skins/winxp/LightCycle.qml",
        displays: "skins/winxp/Displays.qml",
        automation: "skins/winxp/Automation.qml",
        weather: "skins/winxp/Weather.qml",
        forecast: "skins/winxp/Forecast.qml",
        suntimes: "skins/winxp/SunTimes.qml"
    },
    "id:macos": {
        telemetry: "skins/macos/Telemetry.qml",
        curve: "skins/macos/LightCycle.qml",
        displays: "skins/macos/Displays.qml",
        automation: "skins/macos/Automation.qml",
        weather: "skins/macos/Weather.qml",
        forecast: "skins/macos/Forecast.qml",
        suntimes: "skins/macos/SunTimes.qml"
    },
    "id:nothing": {
        telemetry: "skins/nothing/Telemetry.qml",
        curve: "skins/nothing/LightCycle.qml",
        displays: "skins/nothing/Displays.qml",
        automation: "skins/nothing/Automation.qml",
        weather: "skins/nothing/Weather.qml",
        forecast: "skins/nothing/Forecast.qml",
        suntimes: "skins/nothing/SunTimes.qml"
    },
    "id:ps3": {
        telemetry: "skins/ps3/Telemetry.qml",
        curve: "skins/ps3/LightCycle.qml",
        displays: "skins/ps3/Displays.qml",
        automation: "skins/ps3/Automation.qml",
        weather: "skins/ps3/Weather.qml",
        forecast: "skins/ps3/Forecast.qml",
        suntimes: "skins/ps3/SunTimes.qml"
    },
    "id:amiga": {
        telemetry: "skins/amiga/Telemetry.qml",
        curve: "skins/amiga/LightCycle.qml",
        displays: "skins/amiga/Displays.qml",
        automation: "skins/amiga/Automation.qml",
        weather: "skins/amiga/Weather.qml",
        forecast: "skins/amiga/Forecast.qml",
        suntimes: "skins/amiga/SunTimes.qml"
    },
    "id:wii": {
        telemetry: "skins/wii/Telemetry.qml",
        curve: "skins/wii/LightCycle.qml",
        displays: "skins/wii/Displays.qml",
        automation: "skins/wii/Automation.qml",
        weather: "skins/wii/Weather.qml",
        forecast: "skins/wii/Forecast.qml",
        suntimes: "skins/wii/SunTimes.qml"
    },
    "id:commodore64": {
        telemetry: "skins/commodore64/Telemetry.qml",
        curve: "skins/commodore64/LightCycle.qml",
        displays: "skins/commodore64/Displays.qml",
        automation: "skins/commodore64/Automation.qml",
        weather: "skins/commodore64/Weather.qml",
        forecast: "skins/commodore64/Forecast.qml",
        suntimes: "skins/commodore64/SunTimes.qml"
    },
    "id:nds": {
        telemetry: "skins/nds/Telemetry.qml",
        curve: "skins/nds/LightCycle.qml",
        displays: "skins/nds/Displays.qml",
        automation: "skins/nds/Automation.qml",
        weather: "skins/nds/Weather.qml",
        forecast: "skins/nds/Forecast.qml",
        suntimes: "skins/nds/SunTimes.qml"
    }
};

function file(id) {
    return FILES[id] || "";
}

/*! Path (relative to ui/) of the QML file to load for a module id. */
function source(id, themeId, renderer) {
    const base = FILES[id];
    if (!base) {
        return "";
    }
    const keys = ["id:" + themeId, renderer];
    for (let i = 0; i < keys.length; ++i) {
        const skin = SKINS[keys[i]];
        if (skin && skin[id]) {
            return skin[id];
        }
    }
    return "modules/" + base;
}

/*
 * Tab pages (DESIGN.md section 6). The popup's modules are grouped into the
 * pages a tab bar switches between: Brightness (light cycle, displays,
 * automation) and Weather (weather, forecast, sun times). Telemetry is not a
 * page: it is the status line above the tab bar (ABOVE_TABS).
 *
 * FullRepresentation shows the tab bar only when the theme's modules include
 * modules for every page; otherwise one page holds them all, in the theme's
 * order. A module id no page lists belongs to the first page.
 */
var PAGES = ["brightness", "weather"];

var PAGE_OF = {
    curve: "brightness",
    displays: "brightness",
    automation: "brightness",
    weather: "weather",
    forecast: "weather",
    suntimes: "weather"
};

/*! Modules that stay above the tab bar, whatever the theme's order. */
var ABOVE_TABS = ["telemetry"];

/*! The page a module id belongs to. */
function page(id) {
    return PAGE_OF[id] || PAGES[0];
}

/*! The page `delta` places away from `current`, wrapping (+1 next, -1 previous). */
function step(current, delta) {
    const n = PAGES.length;
    const at = Math.max(0, PAGES.indexOf(current));
    return PAGES[((at + delta) % n + n) % n];
}

/*
 * Tab bar skins (components/tabs/), looked up like footer/Parts.js: a theme id
 * key wins, then the renderer. Without one (no daemon theme, or a renderer with
 * no skin) the plain Plasma tab bar is used.
 */
var TABBAR_PLAIN = "components/tabs/PlainTabs.qml";

var TABBAR = {
    "pixel": "components/tabs/PixelTabs.qml",
    "terminal": "components/tabs/TerminalTabs.qml",
    "vector": "components/tabs/InstrumentTabs.qml",
    "id:synthwave84": "skins/synthwave84/Tabs.qml",
    "id:cyberpunk": "skins/cyberpunk/CyberTabs.qml",
    "id:nothing": "skins/nothing/Tabs.qml",
    "id:winxp": "skins/winxp/Tabs.qml",
    "id:ps3": "skins/ps3/Tabs.qml",
    "id:macos": "skins/macos/Tabs.qml",
    "id:amiga": "skins/amiga/Tabs.qml",
    "id:wii": "skins/wii/Tabs.qml",
    "id:nds": "skins/nds/Tabs.qml",
    "id:commodore64": "skins/commodore64/Tabs.qml"
};

/*
 * Popup backdrops: a skin may paint the popup's own fill (a Gradient, loaded by
 * FullRepresentation and handed to the popup Frame) instead of the flat theme
 * background. Keys are "id:<theme id>"; a theme without one keeps the flat fill.
 */
var BACKDROP = {
    "id:synthwave84": "skins/synthwave84/Backdrop.qml",
    "id:ps3": "skins/ps3/Backdrop.qml",
    "id:wii": "skins/wii/Backdrop.qml"
};

/*! Path (relative to ui/) of a theme's popup backdrop, or "" for the flat fill. */
function backdropSource(themeId) {
    return BACKDROP["id:" + themeId] || "";
}

/*
 * Popup overlays: an Item a skin lays over the popup's fill and under its
 * content (FullRepresentation loads it inside the popup Frame, handed the theme).
 * Static art only; keys are "id:<theme id>" as for BACKDROP.
 */
var OVERLAY = {
    "id:ps3": "skins/ps3/Waves.qml",
    "id:commodore64": "skins/commodore64/C64Frame.qml"
};

/*! Path (relative to ui/) of a theme's popup overlay, or "" for none. */
function overlaySource(themeId) {
    return OVERLAY["id:" + themeId] || "";
}

/*! Path (relative to ui/) of the tab bar. Pass themeId "" and renderer "" for no theme. */
function tabBarSource(themeId, renderer) {
    const keys = ["id:" + themeId, renderer];
    for (let i = 0; i < keys.length; ++i) {
        const skin = TABBAR[keys[i]];
        if (skin) {
            return skin;
        }
    }
    return TABBAR_PLAIN;
}

/*
 * Weather icons (components/weathericons/): the panel's sky and the popup's
 * weather headers draw the icon of the theme's own weather style. Keys are
 * "id:<theme id>" (wins) or the renderer name; values are paths relative to ui/.
 * Every file takes the same contract:
 *   required property string condition   daemon condition token ("" unknown)
 *   required property bool night         night variants
 *   required property var theme          resolved tokens (engine/Theme.qml)
 *   property real size                   logical px, square; the file snaps
 *                                        its own art to device pixels
 *   property color surface, ink          what the icon sits on (default: theme bg, fg)
 * A file below its own size threshold draws the generic line glyph and says so
 * in its header. Themes without a key get GlyphIcon (the generic glyph).
 */
var WEATHERICON_DIR = "components/weathericons/";

var WEATHERICON = {
    "id:nothing": WEATHERICON_DIR + "NothingIcon.qml",
    "id:casiodigital": WEATHERICON_DIR + "CasioIcon.qml",
    "id:commodore64": WEATHERICON_DIR + "C64Icon.qml",
    "id:amiga": WEATHERICON_DIR + "AmigaIcon.qml",
    "id:ps3": WEATHERICON_DIR + "Ps3Icon.qml",
    "id:macos": WEATHERICON_DIR + "MacosIcon.qml",
    "id:wii": WEATHERICON_DIR + "WiiWeatherIcon.qml",
    "id:winxp": WEATHERICON_DIR + "WinxpIcon.qml",
    "id:cyberpunk": WEATHERICON_DIR + "CyberpunkIcon.qml",
    "id:synthwave84": WEATHERICON_DIR + "SynthwaveIcon.qml",
    "id:hackergreen": WEATHERICON_DIR + "HackergreenIcon.qml",
    "id:nds": WEATHERICON_DIR + "NdsWeatherIcon.qml",
    "pixel": WEATHERICON_DIR + "PixelIcon.qml",
    "terminal": WEATHERICON_DIR + "TerminalIcon.qml",
    "vector": WEATHERICON_DIR + "InstrumentIcon.qml"
};

var WEATHERICON_GENERIC = WEATHERICON_DIR + "GlyphIcon.qml";

/*! Path (relative to ui/) of the weather icon for a theme id and renderer. */
function weatherIcon(themeId, renderer) {
    const keys = ["id:" + themeId, renderer];
    for (let i = 0; i < keys.length; ++i) {
        const icon = WEATHERICON[keys[i]];
        if (icon) {
            return icon;
        }
    }
    return WEATHERICON_GENERIC;
}
