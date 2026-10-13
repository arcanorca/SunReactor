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
var SKINS = {
    "pixel": {},
    "terminal": {},
    "vector": {},
    "id:braun": {},
    "id:vfd_hifi": {},
    "id:nixie": {}
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
