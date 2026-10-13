/*
 * Headless contact sheet for WeatherGlyph: every condition (day and night) in
 * both styles, on the background of three fixture themes, at 48 and 22 px.
 * Palettes come from tools/fixtures/themes.json. tools/glyph_gallery.sh renders
 * this offscreen to one PNG; the output path follows "--" on the command line.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import "../../package/contents/ui/engine"

Window {
    id: win

    width: 1000
    height: 960
    visible: true
    color: "#202326"
    title: "WeatherGlyph gallery"

    readonly property string outputPath: {
        const args = Qt.application.arguments;
        const at = args.indexOf("--");
        return at >= 0 && at + 1 < args.length ? args[at + 1] : "";
    }

    property var themes: ({})
    property string loadError: ""

    readonly property var columns: [
        { theme: "braun", style: "line" },
        { theme: "braun", style: "solid" },
        { theme: "dracula", style: "line" },
        { theme: "dracula", style: "solid" },
        { theme: "handheld", style: "line" },
        { theme: "handheld", style: "solid" }
    ]

    readonly property var rows: [
        { label: "clear", cond: "clear", night: false },
        { label: "clear, night", cond: "clear", night: true },
        { label: "partly cloudy", cond: "partly_cloudy", night: false },
        { label: "partly cloudy, night", cond: "partly_cloudy", night: true },
        { label: "cloudy", cond: "cloudy", night: false },
        { label: "drizzle", cond: "drizzle", night: false },
        { label: "rain", cond: "rain", night: false },
        { label: "heavy rain", cond: "heavy_rain", night: false },
        { label: "thunderstorm", cond: "thunderstorm", night: false },
        { label: "snow", cond: "snow", night: false },
        { label: "mist", cond: "mist", night: false },
        { label: "fog", cond: "fog", night: false },
        { label: "unknown", cond: "", night: false }
    ]

    function loadThemes() {
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
        win.themes = JSON.parse(xhr.responseText);
    }

    Component.onCompleted: loadThemes()

    Column {
        x: 24
        y: 20
        spacing: 0

        Row {
            Item { width: 176; height: 40 }
            Repeater {
                model: win.columns

                Text {
                    required property var modelData

                    width: 128
                    height: 40
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    text: modelData.theme + " / " + modelData.style
                    color: "#8a9099"
                    font.pixelSize: 13
                }
            }
        }

        Repeater {
            model: win.rows

            Row {
                id: rowItem

                required property var modelData

                Text {
                    width: 176
                    height: 66
                    verticalAlignment: Text.AlignVCenter
                    text: rowItem.modelData.label
                    color: "#c9ced6"
                    font.pixelSize: 14
                }

                Repeater {
                    model: win.columns

                    Item {
                        id: cell

                        required property var modelData

                        readonly property var pal: win.themes[modelData.theme]
                                                   ? win.themes[modelData.theme].palette : ({})

                        width: 128
                        height: 66

                        Rectangle {
                            anchors.fill: parent
                            anchors.margins: 3
                            radius: 6
                            color: cell.pal.bg !== undefined ? cell.pal.bg : "#101010"

                            WeatherGlyph {
                                x: 12
                                y: (parent.height - height) / 2
                                size: 48
                                condition: rowItem.modelData.cond
                                night: rowItem.modelData.night
                                glyphStyle: cell.modelData.style
                                palette: cell.pal
                            }

                            WeatherGlyph {
                                x: 74
                                y: (parent.height - height) / 2
                                size: 22
                                condition: rowItem.modelData.cond
                                night: rowItem.modelData.night
                                glyphStyle: cell.modelData.style
                                palette: cell.pal
                            }
                        }
                    }
                }
            }
        }
    }

    Text {
        x: 24
        y: win.height - 34
        color: "#7d848e"
        font.pixelSize: 12
        text: "48 px and 22 px  |  device pixel ratio " + win.devicePixelRatio
              + (win.loadError.length ? "  |  " + win.loadError : "")
    }

    Timer {
        interval: 600
        running: true
        onTriggered: {
            if (win.outputPath.length === 0) {
                console.log("GLYPH_GALLERY no output path after --");
                Qt.quit();
                return;
            }
            win.contentItem.grabToImage(function (result) {
                const ok = result.saveToFile(win.outputPath);
                console.log("GLYPH_GALLERY " + (ok ? "saved " : "FAILED ") + win.outputPath
                            + " dpr " + win.devicePixelRatio);
                Qt.quit();
            });
        }
    }
}
