/*
 * Forecast, instrument family: the intervals the daemon already fetched for
 * its own policy, as a dense table with a deterministic temperature trace.
 *
 *   FORECAST ────────────────────────────────────────────────────
 *   TIME    15:00   16:00   17:00   18:00   19:00
 *   TEMP     15°  ─  14°  ─  13°  ─  13°  ─  12°
 *   SKY     P.CLDY  CLOUDY  RAIN    CLOUDY  P.CLDY
 *   CLOUD   ▮▮▮▮▯   ▮▮▮▮▯   ...                       (cover, the policy input)
 *
 * The number of columns follows the width; no sprites (words and numbers
 * read better on a panel than coloured icons).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }

    TextMetrics {
        id: rowMetrics
        font: root.style.legendFont
        text: "CLOUD"
    }
    TextMetrics {
        id: timeMetrics
        font: root.style.dataSmallFont
        text: "00:00 PM"
    }

    readonly property real labelWidth: style.snap(rowMetrics.width + style.gap2)
    /*! Columns that fit, 3..6. */
    readonly property int slots: Math.max(3, Math.min(6, Math.floor((width - labelWidth) / (timeMetrics.width + style.gap2 * 1.5))))

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

    function shortCondition(condition) {
        switch (condition) {
        case "clear":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Clear");
        case "partly_cloudy":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "P.cloudy");
        case "cloudy":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Cloudy");
        case "drizzle":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Drizzle");
        case "rain":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Rain");
        case "heavy_rain":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "H.rain");
        case "thunderstorm":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Storm");
        case "snow":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Snow");
        case "mist":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Mist");
        case "fog":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Fog");
        case "atmospheric":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "Haze");
        default:
            return "—";
        }
    }

    readonly property real tMin: points.length ? Math.min(...points.map(p => p.temperatureC)) : 0
    readonly property real tMax: points.length ? Math.max(...points.map(p => p.temperatureC)) : 1

    spacing: style.rowGap
    // Width comes from the popup; never from the table (columns follow width).
    implicitWidth: 0

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.domain, "Instrument legend over the weather forecast", "Forecast")
    }

    GridLayout {
        Layout.fillWidth: true
        columns: root.points.length + 1
        columnSpacing: 0
        rowSpacing: root.style.snap(root.style.gap * 0.75)

        // Row: time
        Text {
            Layout.preferredWidth: root.labelWidth
            text: i18ndc(root.domain, "Forecast row legend", "Time")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Repeater {
            model: root.points
            delegate: Text {
                required property var modelData
                Layout.preferredWidth: 1
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: Format.timeText(modelData.epochS)
                textFormat: Text.PlainText
                font: root.style.dataSmallFont
                color: root.style.inkMuted
                elide: Text.ElideRight
            }
        }

        // Row: temperature (with a trace behind the numbers)
        Text {
            Layout.preferredWidth: root.labelWidth
            text: i18ndc(root.domain, "Forecast row legend", "Temp")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Repeater {
            model: root.points
            delegate: Text {
                required property var modelData
                Layout.preferredWidth: 1
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                             Math.round(modelData.temperatureC))
                textFormat: Text.PlainText
                font: root.style.dataFont
                color: root.style.ink
            }
        }

        // Row: temperature trace (deterministic sparkline, min..max of the window)
        Text {
            Layout.preferredWidth: root.labelWidth
            text: ""
        }
        Item {
            id: spark

            Layout.columnSpan: Math.max(1, root.points.length)
            Layout.fillWidth: true
            implicitHeight: root.style.snap(root.style.sizeLegend * 1.4)

            readonly property real span: Math.max(1, root.tMax - root.tMin)
            function px(i) {
                return (i + 0.5) * width / Math.max(1, root.points.length);
            }
            function py(t) {
                return root.style.hairline + (1 - (t - root.tMin) / span) * (height - root.style.hairline * 3);
            }

            Shape {
                anchors.fill: parent
                preferredRendererType: Shape.CurveRenderer
                ShapePath {
                    strokeColor: root.style.inkMuted
                    strokeWidth: root.style.line
                    fillColor: "transparent"
                    joinStyle: ShapePath.RoundJoin
                    PathPolyline {
                        path: root.points.map((p, i) => Qt.point(spark.px(i), spark.py(p.temperatureC)))
                    }
                }
            }
            Repeater {
                model: root.points
                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    width: root.style.snap(3)
                    height: width
                    x: spark.px(index) - width / 2
                    y: spark.py(modelData.temperatureC) - height / 2
                    color: root.style.ink
                }
            }
        }

        // Row: sky
        Text {
            Layout.preferredWidth: root.labelWidth
            text: i18ndc(root.domain, "Forecast row legend", "Sky")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Repeater {
            model: root.points
            delegate: Text {
                required property var modelData
                Layout.preferredWidth: 1
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                text: root.shortCondition(modelData.condition)
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.ink
                elide: Text.ElideRight
            }
        }

        // Row: cloud cover micro-bars (the input the policy uses)
        Text {
            Layout.preferredWidth: root.labelWidth
            text: i18ndc(root.domain, "Forecast row legend: cloud cover", "Cloud")
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkMuted
        }
        Repeater {
            model: root.points
            delegate: Item {
                id: cover

                required property var modelData
                readonly property int pct: modelData.cloudCoverPercent !== undefined ? modelData.cloudCoverPercent : -1

                Layout.preferredWidth: 1
                Layout.fillWidth: true
                implicitHeight: coverText.implicitHeight

                Row {
                    anchors.centerIn: parent
                    spacing: root.style.gap

                    Item {
                        id: bar
                        anchors.verticalCenter: parent.verticalCenter
                        visible: cover.pct >= 0
                        width: root.style.snap(root.style.unit * 1.4)
                        height: root.style.snap(root.style.sizeLegend * 0.55)

                        Rectangle {
                            anchors.fill: parent
                            color: root.style.ghost
                        }
                        Rectangle {
                            width: root.style.snap(bar.width * cover.pct / 100)
                            height: parent.height
                            color: root.style.inkMuted
                        }
                    }
                    Text {
                        id: coverText
                        anchors.verticalCenter: parent.verticalCenter
                        text: cover.pct >= 0 ? String(cover.pct) : "—"
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.inkMuted
                    }
                }
            }
        }
    }
}
