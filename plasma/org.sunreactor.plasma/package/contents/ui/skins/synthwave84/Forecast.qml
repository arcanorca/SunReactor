/*
 * Forecast, synthwave84: the intervals the daemon already fetched for its own
 * policy, in a neon card. Each column has the time, a cyan line glyph and the
 * temperature; a pink trace joins the temperatures across the card.
 *
 *   FORECAST ───────────────────────────────────────────
 *   ·  14°    ·  13°   ·  12°
 *   15:00     16:00    17:00
 *   ◌         ◌        ◌
 *
 * The number of columns follows the width. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }

    /*! Columns that fit, 3..6. */
    readonly property int slots: Math.max(3, Math.min(6, Math.floor(width / (style.unit * 3.4))))
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === slots) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    /*! The temperature range the trace is scaled to. */
    readonly property real tMin: points.length > 0 ? Math.min(...points.map(p => p.temperatureC)) : 0
    readonly property real tMax: points.length > 0 ? Math.max(...points.map(p => p.temperatureC)) : 1

    spacing: style.rowGap

    Heading {
        style: root.style
        text: i18ndc(root.domain, "Synthwave heading of the forecast", "Forecast")
    }

    NeonCard {
        Layout.fillWidth: true
        style: root.style
        neon: root.style.pink

        Item {
            id: strip

            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.unit * 4.3)

            readonly property int count: root.points.length
            readonly property real colW: width / Math.max(1, count)
            /*! The trace band (top) and the glyph and time rows (below it). */
            readonly property real traceH: root.style.unit * 2.0
            readonly property real glyphTop: root.style.unit * 2.1

            function xOf(i) {
                return colW * (i + 0.5);
            }
            function yOf(t) {
                const span = Math.max(1, root.tMax - root.tMin);
                // Room above the warmest point for its label.
                const top = root.style.unit * 0.9;
                const bottom = traceH - root.style.unit * 0.3;
                return top + (1 - (t - root.tMin) / span) * (bottom - top);
            }

            // The trace through the temperatures.
            Shape {
                anchors.fill: parent
                visible: strip.count > 1
                preferredRendererType: Shape.CurveRenderer

                ShapePath {
                    strokeColor: root.style.pink
                    strokeWidth: root.style.line2
                    fillColor: "transparent"
                    joinStyle: ShapePath.RoundJoin
                    capStyle: ShapePath.RoundCap
                    PathPolyline {
                        path: root.points.map((p, i) => Qt.point(strip.xOf(i), strip.yOf(p.temperatureC)))
                    }
                }
            }

            Repeater {
                model: root.points

                delegate: Item {
                    id: column

                    required property var modelData
                    required property int index

                    x: strip.colW * index
                    y: 0
                    width: strip.colW
                    height: strip.height

                    // Temperature and its dot on the trace.
                    Rectangle {
                        x: Math.round(column.width / 2 - width / 2)
                        y: Math.round(strip.yOf(column.modelData.temperatureC) - height / 2)
                        width: root.style.snap(root.style.unit * 0.3)
                        height: width
                        radius: width / 2
                        color: root.style.ink
                        border.width: root.style.line2
                        border.color: root.style.pink
                    }
                    Text {
                        x: Math.round(column.width / 2 - width / 2)
                        y: Math.round(strip.yOf(column.modelData.temperatureC) - height - root.style.gap)
                        text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                     Math.round(column.modelData.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.dataFont
                        color: root.style.ink
                    }

                    // The sky: a line glyph in cyan, the sun in pink.
                    WeatherGlyph {
                        x: Math.round((column.width - width) / 2)
                        y: strip.glyphTop
                        size: root.style.snap(root.style.unit * 1.3)
                        condition: column.modelData.condition
                        glyphStyle: "line"
                        palette: root.style.glyphPalette
                    }

                    // The hour.
                    Text {
                        x: Math.round((column.width - width) / 2)
                        y: strip.glyphTop + root.style.unit * 1.4
                        text: Format.timeText(column.modelData.epochS)
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.cyan
                    }
                }
            }
        }
    }
}
