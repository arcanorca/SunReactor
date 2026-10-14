/*
 * Headless contact sheet for the dot-matrix primitives (plugin/dotmatrix.cpp):
 * DotText at two sizes, DotBar at three values, DotRing, DotCurve on a synthetic
 * day, and every DotIcon condition with its night variants, on black. Colours
 * come from the nothing entry of tools/fixtures/themes.json. tools/dot_gallery.sh
 * renders this offscreen to one PNG; the output path follows "--".
 */

import QtQuick
import QtQuick.Window
import org.sunreactor.plasma

Window {
    id: win

    width: 1000
    height: 1040
    visible: true
    color: "#000000"
    title: "Dot matrix gallery"

    readonly property string outputPath: {
        const args = Qt.application.arguments;
        const at = args.indexOf("--");
        return at >= 0 && at + 1 < args.length ? args[at + 1] : "";
    }

    property var themes: ({})
    property string loadError: ""

    readonly property var palette: themes.nothing ? themes.nothing.palette : ({})
    readonly property color fg: palette.fg !== undefined ? palette.fg : "#ffffff"
    readonly property color accent: palette.accent !== undefined ? palette.accent : "#d71921"
    readonly property color muted: palette.text_muted !== undefined ? palette.text_muted : "#8a8a8a"

    readonly property int pitch: 5

    readonly property var icons: [
        { condition: "clear", night: false, label: "clear" },
        { condition: "clear", night: true, label: "clear night" },
        { condition: "partly_cloudy", night: false, label: "partly cloudy" },
        { condition: "partly_cloudy", night: true, label: "partly night" },
        { condition: "cloudy", night: false, label: "cloudy" },
        { condition: "drizzle", night: false, label: "drizzle" },
        { condition: "rain", night: false, label: "rain" },
        { condition: "heavy_rain", night: false, label: "heavy rain" },
        { condition: "thunderstorm", night: false, label: "thunderstorm" },
        { condition: "snow", night: false, label: "snow" },
        { condition: "mist", night: false, label: "mist" },
        { condition: "fog", night: false, label: "fog" },
        { condition: "atmospheric", night: false, label: "atmospheric" },
        { condition: "", night: false, label: "unknown" }
    ]

    // Synthetic day: 06:00 to 21:00, a half-sine with a weather dip around 13:00.
    // The no-weather series sits above the weather one, as in client.dayCurve.
    function daySamples(withWeather) {
        const out = [];
        for (let t = 21600; t <= 75600; t += 900) {
            const x = (t - 21600) / 54000;
            const base = 6 + 86 * Math.sin(Math.PI * x);
            const dip = 14 * Math.exp(-Math.pow((x - 0.45) / 0.12, 2));
            out.push({ t: t, pct: withWeather ? Math.max(0, base - dip) : base });
        }
        return out;
    }
    readonly property var weatherSamples: daySamples(true)
    readonly property var plainSamples: daySamples(false)

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

    // grabToImage keeps only the item tree: the black must be an item.
    Rectangle {
        anchors.fill: win.contentItem
        color: "#000000"
    }

    Column {
        x: 36
        y: 32
        spacing: 34

        Text {
            color: win.muted
            font.family: "monospace"
            font.pixelSize: 13
            text: "DOT MATRIX  |  pitch " + win.pitch + "  |  device pixel ratio " + win.devicePixelRatio
                  + (win.loadError.length ? "  |  " + win.loadError : "")
        }

        // DotText, two sizes.
        Column {
            spacing: 14

            Row {
                spacing: 40

                DotText {
                    text: "39%"
                    size: 3
                    pitch: win.pitch
                    litColor: win.fg
                    accentColor: win.accent
                    dimColor: Qt.alpha(win.fg, 0.12)
                }

                DotText {
                    text: "15°"
                    size: 3
                    pitch: win.pitch
                    litColor: win.fg
                    accentColor: win.accent
                    dimColor: Qt.alpha(win.fg, 0.12)
                }
            }

            DotText {
                text: "NEXT 16:20 -> 23%"
                size: 1
                letterSpacing: 1
                accentIndex: 0
                pitch: win.pitch
                litColor: win.fg
                accentColor: win.accent
                dimColor: Qt.alpha(win.fg, 0.12)
            }

            DotText {
                text: "auto  ·  manual  ·  paused  ·  ÇĞİÖŞÜ"
                size: 1
                letterSpacing: 1
                pitch: win.pitch
                litColor: win.fg
                accentColor: win.accent
                dimColor: Qt.alpha(win.fg, 0.12)
            }
        }

        // DotBar at three values, with the red thumb.
        Column {
            spacing: 12

            Repeater {
                model: [0.2, 0.6, 1.0]

                Row {
                    id: barRow

                    required property real modelData

                    spacing: 22

                    DotBar {
                        value: barRow.modelData
                        cols: 40
                        rows: 1
                        thumbAccent: true
                        pitch: win.pitch
                        litColor: win.fg
                        accentColor: win.accent
                        dimColor: Qt.alpha(win.fg, 0.12)
                        anchors.verticalCenter: parent.verticalCenter
                    }

                    Text {
                        color: win.muted
                        font.family: "monospace"
                        font.pixelSize: 13
                        text: Math.round(barRow.modelData * 100) + "%"
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
            }
        }

        // DotRing: two gauges with the value inside.
        Row {
            spacing: 60

            Repeater {
                model: [0.25, 0.7]

                Item {
                    id: ringCell

                    required property real modelData

                    width: ring.implicitWidth
                    height: ring.implicitHeight

                    DotRing {
                        id: ring
                        value: ringCell.modelData
                        diameter: 21
                        dots: 32
                        pitch: win.pitch
                        litColor: win.fg
                        accentColor: win.accent
                        dimColor: Qt.alpha(win.fg, 0.12)
                    }

                    DotText {
                        anchors.centerIn: ring
                        text: Math.round(ringCell.modelData * 100) + ""
                        size: 2
                        pitch: 4
                        litColor: win.fg
                        accentColor: win.accent
                        dimColor: "transparent"
                    }
                }
            }
        }

        // DotCurve: a column-of-dots day with the weather cost above the lit area.
        DotCurve {
            samples: win.weatherSamples
            samplesNoWeather: win.plainSamples
            t0: 21600
            t1: 75600
            nowT: 50400
            maxPct: 100
            cols: 96
            rows: 16
            pitch: win.pitch
            litColor: win.fg
            accentColor: win.accent
            dimColor: Qt.alpha(win.fg, 0.12)
            ghostColor: Qt.alpha(win.fg, 0.5)
        }

        // DotIcon: every condition, night variants included.
        Grid {
            columns: 7
            columnSpacing: 26
            rowSpacing: 26

            Repeater {
                model: win.icons

                Column {
                    id: iconCell

                    required property var modelData

                    spacing: 8

                    DotIcon {
                        condition: iconCell.modelData.condition
                        night: iconCell.modelData.night
                        pitch: win.pitch
                        litColor: win.fg
                        accentColor: win.accent
                        dimColor: Qt.alpha(win.fg, 0.12)
                    }

                    Text {
                        color: win.muted
                        font.family: "monospace"
                        font.pixelSize: 12
                        text: iconCell.modelData.label
                    }
                }
            }
        }
    }

    Timer {
        interval: 600
        running: true
        onTriggered: {
            if (win.outputPath.length === 0) {
                console.log("DOT_GALLERY no output path after --");
                Qt.quit();
                return;
            }
            win.contentItem.grabToImage(function (result) {
                const ok = result.saveToFile(win.outputPath);
                console.log("DOT_GALLERY " + (ok ? "saved " : "FAILED ") + win.outputPath
                            + " dpr " + win.devicePixelRatio);
                Qt.quit();
            });
        }
    }
}
