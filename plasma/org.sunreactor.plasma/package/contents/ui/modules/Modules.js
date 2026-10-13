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
    "id:nothing": {
        telemetry: "skins/nothing/Telemetry.qml",
        curve: "skins/nothing/LightCycle.qml",
        displays: "skins/nothing/Displays.qml",
        automation: "skins/nothing/Automation.qml",
        weather: "skins/nothing/Weather.qml",
        forecast: "skins/nothing/Forecast.qml",
        suntimes: "skins/nothing/SunTimes.qml"
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
    "id:nothing": "skins/nothing/Tabs.qml"
};

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
