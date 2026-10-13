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

function file(id) {
    return FILES[id] || "";
}
