/*
 * Headless design sheet for a theme's registry weather icon: every condition by
 * day and by night, at each requested size, on the theme's own background. The
 * icon is the one the widget shows (ThemedWeatherIcon -> Modules.weatherIcon), so
 * the sheet follows the registry. tools/icon_sheet.sh renders this offscreen to one
 * PNG per device pixel ratio; the arguments follow "--" on the command line:
 *
 *   -- OUT.png THEME_ID "22 32 48 96"
 *
 * The theme is read from tools/fixtures/themes.json (snake_case palette keys are
 * converted to the camelCase the Theme engine expects, as the plugin does).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import "../../package/contents/ui/engine"
import "../../package/contents/ui/components/weathericons"

Window {
    id: win

    readonly property var argv: {
        const args = Qt.application.arguments;
        const at = args.indexOf("--");
        return at >= 0 ? args.slice(at + 1) : [];
    }
    readonly property string outputPath: argv.length > 0 ? argv[0] : ""
    readonly property string themeId: argv.length > 1 ? argv[1] : "nds"
    readonly property var sizes: (argv.length > 2 ? argv[2] : "22 32 48 96").split(/\s+/)
                                     .filter(s => s.length > 0).map(Number)

    readonly property var conditions: [
        { label: "clear", cond: "clear" },
        { label: "partly cloudy", cond: "partly_cloudy" },
        { label: "cloudy", cond: "cloudy" },
        { label: "drizzle", cond: "drizzle" },
        { label: "rain", cond: "rain" },
        { label: "heavy rain", cond: "heavy_rain" },
        { label: "thunderstorm", cond: "thunderstorm" },
        { label: "snow", cond: "snow" },
        { label: "mist", cond: "mist" },
        { label: "atmospheric", cond: "atmospheric" },
        { label: "fog", cond: "fog" },
        { label: "unknown", cond: "" }
    ]
    /*! Day columns, then night columns, one per size each. */
    readonly property var columns: {
        const out = [];
        for (const night of [false, true]) {
            for (const s of sizes) {
                out.push({ night: night, size: s });
            }
        }
        return out;
    }

    readonly property real cellW: Math.max(96, (sizes.length ? Math.max(...sizes) : 48) + 28)
    readonly property real cellH: cellW
    readonly property real labelW: 150

    width: labelW + columns.length * cellW + 32
    height: 96 + conditions.length * cellH + 40
    visible: true
    color: "#202326"
    title: "Weather icon sheet: " + themeId

    property var fixtures: ({})
    property string loadError: ""

    readonly property var spec: {
        const entry = fixtures[themeId];
        if (!entry || !entry.palette) {
            return ({});
        }
        const palette = {};
        for (const key of Object.keys(entry.palette)) {
            palette[camel(key)] = entry.palette[key];
        }
        return { id: entry.id, name: entry.name, family: entry.family,
                 palette: palette, widget: entry.widget || {} };
    }

    function camel(key) {
        return key.replace(/_([a-z])/g, (m, c) => c.toUpperCase());
    }

    function loadFixtures() {
        const xhr = new XMLHttpRequest();
        xhr.open("GET", Qt.resolvedUrl("../fixtures/themes.json"), false);
        try {
            xhr.send();
        } catch (e) {
            win.loadError = String(e);
            return;
        }
        if (xhr.responseText.length === 0) {
            win.loadError = "empty themes.json";
            return;
        }
        win.fixtures = JSON.parse(xhr.responseText);
    }

    Component.onCompleted: loadFixtures()

    /*! "auto" (default), "modern", "line", "solid" or "pixel": the Weather icons setting. */
    readonly property string styleArg: argv.length > 3 && argv[3].length > 0 ? argv[3] : "auto"

    Theme {
        id: sheetTheme

        spec: win.spec
        overrides: ({ weatherIconStyle: win.styleArg })
    }

    Column {
        x: 16
        y: 16
        spacing: 0

        Row {
            Item { width: win.labelW; height: 40 }
            Repeater {
                model: win.columns

                Text {
                    required property var modelData

                    width: win.cellW
                    height: 40
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    text: (modelData.night ? "night " : "day ") + modelData.size
                    color: "#9aa1ab"
                    font.pixelSize: 13
                }
            }
        }

        Repeater {
            model: win.conditions

            Row {
                id: line

                required property var modelData

                Text {
                    width: win.labelW
                    height: win.cellH
                    verticalAlignment: Text.AlignVCenter
                    text: line.modelData.label
                    color: "#c9ced6"
                    font.pixelSize: 14
                }

                Repeater {
                    model: win.columns

                    Rectangle {
                        id: cell

                        required property var modelData

                        width: win.cellW
                        height: win.cellH
                        color: sheetTheme.bg
                        border.width: 1
                        border.color: Qt.rgba(0, 0, 0, 0.08)

                        ThemedWeatherIcon {
                            anchors.centerIn: parent
                            theme: sheetTheme
                            condition: line.modelData.cond
                            night: cell.modelData.night
                            size: cell.modelData.size
                        }
                    }
                }
            }
        }
    }

    Text {
        x: 16
        y: win.height - 30
        color: "#7d848e"
        font.pixelSize: 12
        text: "theme " + win.themeId + " (" + win.styleArg + ")  |  device pixel ratio " + win.devicePixelRatio
              + (win.loadError.length ? "  |  " + win.loadError : "")
              + (sheetTheme.available ? "" : "  |  unknown theme id")
    }

    Timer {
        interval: 800
        running: true
        onTriggered: {
            if (win.outputPath.length === 0) {
                console.log("ICON_SHEET no output path after --");
                Qt.quit();
                return;
            }
            win.contentItem.grabToImage(function (result) {
                const ok = result.saveToFile(win.outputPath);
                console.log("ICON_SHEET " + (ok ? "saved " : "FAILED ") + win.outputPath
                            + " theme " + win.themeId + " dpr " + win.devicePixelRatio);
                Qt.quit();
            });
        }
    }
}
