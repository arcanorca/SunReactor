/*
 * Forecast, cyberpunk: the intervals the daemon already fetched, one column per
 * interval, as a HUD table. Each column: the time, a line glyph in cyan and
 * yellow (night variants for night), the temperature, the sky word, and the cloud
 * cover (the policy input) as a small cell bar and bracketed number.
 *
 *   FORECAST ──────────────────────────────────────────────────
 *   15:00 │ 16:00 │ 17:00 │ 18:00 │ 19:00
 *   (glyph)  ...
 *   15°   │ 14°   │ 13°
 *   P.CLDY│ CLOUDY│ RAIN
 *   ▰▰▱▱▱ [40]
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

    /*! Columns that fit the width, 3..6. */
    readonly property int slots: Math.max(3, Math.min(6, Math.floor(width / (style.unit * 4.4))))

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

    spacing: style.rowGap

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.domain, "Panel title: the weather forecast", "Forecast")
        tag: root.points.length > 0 ? i18ndc(root.domain, "Number of forecast columns", "%1 SLOTS", root.points.length) : ""

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
                        anchors.leftMargin: column.index > 0 ? root.style.gap : 0
                        spacing: root.style.gap

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: Format.timeText(column.modelData.epochS)
                            textFormat: Text.PlainText
                            font: root.style.techFont
                            color: root.style.cyan
                        }
                        WeatherGlyph {
                            Layout.alignment: Qt.AlignHCenter
                            condition: column.modelData.condition
                            night: column.modelData.isNight === true
                            glyphStyle: "line"
                            palette: root.glyphColours
                            size: Math.round(root.style.unit * 2.2)
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                         Math.round(column.modelData.temperatureC))
                            textFormat: Text.PlainText
                            font: root.style.numFont
                            color: root.style.yellow
                        }
                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.maximumWidth: column.width
                            text: root.shortCondition(column.modelData.condition)
                            textFormat: Text.PlainText
                            font: root.style.labelFont
                            color: root.style.ink
                            elide: Text.ElideRight
                        }
                        CellBar {
                            Layout.fillWidth: true
                            Layout.preferredHeight: root.style.snap(root.style.unit * 1.2)
                            style: root.style
                            cells: 5
                            value: column.pct
                            litColor: root.style.cyan
                        }
                        Bracket {
                            Layout.alignment: Qt.AlignHCenter
                            style: root.style
                            value: column.pct >= 0 ? String(column.pct) : "--"
                            font: root.style.techFont
                            color: root.style.ink
                            bracketColor: root.style.muted
                        }
                    }
                }
            }
        }
    }
}
