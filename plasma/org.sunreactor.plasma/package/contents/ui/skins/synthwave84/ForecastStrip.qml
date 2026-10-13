/*
 * The forecast strip, synthwave84, without a card: the next five intervals the
 * daemon already fetched. A pink trace joins the temperatures across the strip;
 * under it, each column has a cyan line glyph (the sun in pink) and the hour.
 * Framed by Forecast.qml's card, or hosted by the Weather card.
 *
 *    15°  14°  13°  13°  12°
 *    ·────·────·────·────·
 *    ◌    ◌    ◌    ◌    ◌
 *   15:00 16:00 17:00 18:00 19:00
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

    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === 5) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0
    /*! The temperature range the trace is scaled to. */
    readonly property real tMin: points.length > 0 ? Math.min(...points.map(p => p.temperatureC)) : 0
    readonly property real tMax: points.length > 0 ? Math.max(...points.map(p => p.temperatureC)) : 1

    spacing: 0

    Item {
        id: strip

        Layout.fillWidth: true
        implicitHeight: root.style.snap(root.style.unit * 3.5)
        readonly property int count: root.points.length
        readonly property real colW: width / Math.max(1, count)
        /*! The trace band (top) and the glyph and hour rows (below it). */
        readonly property real traceH: root.style.unit * 1.6
        readonly property real glyphTop: root.style.unit * 1.75

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
                    y: Math.round(strip.yOf(column.modelData.temperatureC) - height - root.style.gap * 0.5)
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
                    size: root.style.snap(root.style.unit * 1.0)
                    condition: column.modelData.condition
                    glyphStyle: "line"
                    palette: root.style.glyphPalette
                }
                // The hour.
                Text {
                    x: Math.round((column.width - width) / 2)
                    y: strip.glyphTop + root.style.unit * 1.2
                    text: Format.timeText(column.modelData.epochS)
                    textFormat: Text.PlainText
                    font: root.style.dataSmallFont
                    color: root.style.cyan
                }
            }
        }
    }
}
