/*
 * The forecast strip, cyberpunk, without a frame: the next five intervals the
 * daemon already fetched, one column each. A column: the hour, a small line glyph
 * (cyan sky, yellow sun and moon) beside the temperature, the sky word, and the
 * cloud cover (the policy input) as a cell bar with its bracketed number. Framed
 * by Forecast.qml, or hosted by the Weather panel.
 *
 *   15:00     16:00     17:00     18:00     19:00
 *   ◌ 15°     ◌ 14°     ◌ 13°     ◌ 13°     ◌ 12°
 *   P.CLDY    CLOUDY    RAIN      RAIN      CLEAR
 *   ▰▰▱▱▱     ▰▰▰▱▱     ▰▰▰▰▱     ▰▰▰▰▰     ▱▱▱▱▱
 *   [40]      [60]      [80]      [90]      [05]
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
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
    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    /*! Line glyph colours: sky in cyan, sun and moon in yellow. */
    readonly property var glyphColours: theme.glyphColours(theme.bg, theme.secondaryAccent)

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

    function shortCondition(condition) {
        switch (condition) {
        case "clear":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "CLEAR");
        case "partly_cloudy":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "P.CLDY");
        case "cloudy":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "CLOUDY");
        case "drizzle":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "DRIZZLE");
        case "rain":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "RAIN");
        case "heavy_rain":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "H.RAIN");
        case "thunderstorm":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "STORM");
        case "snow":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "SNOW");
        case "mist":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "MIST");
        case "fog":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "FOG");
        case "atmospheric":
            return i18ndc(domain, "Short weather condition (max ~7 letters)", "HAZE");
        default:
            return "--";
        }
    }

    spacing: 0

    RowLayout {
        Layout.fillWidth: true
        spacing: 0

        Repeater {
            model: root.points

            delegate: Item {
                id: column

                required property var modelData
                required property int index
                readonly property int pct: modelData.cloudCoverPercent !== undefined ? modelData.cloudCoverPercent : -1

                Layout.fillWidth: true
                implicitHeight: body.implicitHeight

                // Hairline between the columns.
                Rectangle {
                    visible: column.index > 0
                    x: 0
                    y: 0
                    width: root.style.hairline
                    height: column.height
                    color: root.style.ruleFaint
                }

                ColumnLayout {
                    id: body

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.leftMargin: column.index > 0 ? root.style.gap * 0.5 : 0
                    spacing: root.style.snap(root.style.unit * 0.2)

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: Format.timeText(column.modelData.epochS)
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.cyan
                    }
                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        spacing: root.style.gap * 0.5

                        WeatherGlyph {
                            Layout.alignment: Qt.AlignVCenter
                            condition: column.modelData.condition
                            night: column.modelData.isNight === true
                            glyphStyle: "line"
                            palette: root.glyphColours
                            size: Math.round(root.style.unit * 1.0)
                        }
                        Text {
                            Layout.alignment: Qt.AlignVCenter
                            text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                         Math.round(column.modelData.temperatureC))
                            textFormat: Text.PlainText
                            font: root.style.numFont
                            color: root.style.yellow
                        }
                    }
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        Layout.maximumWidth: column.width
                        text: root.shortCondition(column.modelData.condition)
                        textFormat: Text.PlainText
                        font: root.style.tinyFont
                        color: root.style.ink
                        elide: Text.ElideRight
                    }
                    CellBar {
                        Layout.fillWidth: true
                        style: root.style
                        cells: 5
                        value: column.pct
                        litColor: root.style.cyan
                    }
                    Bracket {
                        Layout.alignment: Qt.AlignHCenter
                        style: root.style
                        value: column.pct >= 0 ? String(column.pct) : "--"
                        font: root.style.tinyFont
                        color: root.style.ink
                        bracketColor: root.style.muted
                    }
                }
            }
        }
    }
}
