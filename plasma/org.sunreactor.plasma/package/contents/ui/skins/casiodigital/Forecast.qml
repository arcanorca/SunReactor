/*
 * Forecast, casiodigital: up to five upcoming hours as evenly spaced readouts
 * separated by faint LCD hairlines, each the hour in print, the condition as a
 * monoline glyph in the ink, and the temperature in seven segments. Drops slots
 * cleanly when the glass is too narrow rather than squeezing them together.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../Format.js" as Format
import "Casio.js" as Casio
import "../../modules/ForecastRules.js" as ForecastRules

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CasioStyle style: CasioStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    property int slots: 5

    readonly property real iconSize: style.snap(style.unit * 1.5)
    readonly property bool has12hSuffix: Casio.clockParts(Format.timeText(client.nowEpochS)).suffix.length > 0
    readonly property real minSlotWidth: Math.max(
        style.snap(iconSize + style.gapWide * 2),
        style.snap(Math.max(hourMetrics.advanceWidth, sampleTempWidth) + style.gapWide)
    )
    readonly property real sampleTempWidth: Casio.seven("88", style.digitSmall, style.dpr).width
        + style.snap(style.gap * 0.5) + degMetrics.advanceWidth
    readonly property int shownSlots: row.width > 0
        ? Math.max(1, Math.min(slots, Math.floor(row.width / Math.max(1, minSlotWidth))))
        : slots
    readonly property var points: ForecastRules.upcoming(client.forecast, client.nowEpochS, shownSlots)

    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled && points.length > 0

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    TextMetrics {
        id: hourMetrics

        font: root.style.cap(root.style.sizeCap)
        text: root.has12hSuffix ? "12 PM" : "23"
    }

    TextMetrics {
        id: degMetrics

        font: root.style.cap(root.style.sizeCap)
        text: "°"
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Forecast")
        trailing: i18ndc(root.domain, "Forecast horizon in hours", "%1 H", root.points.length)

        RowLayout {
            id: row

            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: root.points

                delegate: Item {
                    id: slot

                    required property var modelData
                    required property int index

                    // The hour only ("3 PM", or "15" in a 24-hour locale): five of them must fit.
                    readonly property var clock: Casio.clockParts(Format.timeText(modelData.epochS))
                    readonly property string hourText: clock.digits.split(":")[0].trim()
                        + (clock.suffix.length > 0 ? " " + clock.suffix : "")

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    Layout.minimumWidth: 0
                    implicitWidth: slotCol.implicitWidth + root.style.gapWide
                    implicitHeight: slotCol.implicitHeight

                    Rectangle {
                        visible: slot.index > 0
                        x: 0
                        y: root.style.snap((parent.height - height) / 2)
                        width: root.style.hairline
                        height: root.style.snap(parent.height * 0.82)
                        color: root.style.inkFaint
                    }

                    ColumnLayout {
                        id: slotCol

                        x: root.style.snap((parent.width - width) / 2)
                        y: 0
                        spacing: root.style.gap

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: slot.hourText
                            textFormat: Text.PlainText
                            font: root.style.cap(root.style.sizeCap)
                            color: root.style.inkMuted
                        }

                        ThemedWeatherIcon {
                            theme: root.theme
                            size: root.iconSize
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight === true
                            surface: root.style.glass
                            ink: root.style.ink
                            Layout.alignment: Qt.AlignHCenter
                            WeatherGlyph {
                                anchors.centerIn: parent
                                size: root.iconSize
                                condition: slot.modelData.condition
                                night: slot.modelData.isNight === true
                                glyphStyle: "line"
                                palette: root.style.glyphPalette
                            }
                        }

                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            spacing: root.style.snap(root.style.gap * 0.5)

                            CasioDigits {
                                style: root.style
                                size: root.style.digitSmall
                                text: Casio.numberText(Format.tempValue(slot.modelData.temperatureC, root.client.isFahrenheit), 2)
                            }
                            Text {
                                Layout.alignment: Qt.AlignTop
                                text: "°"
                                textFormat: Text.PlainText
                                font: root.style.cap(root.style.sizeCap)
                                color: root.style.inkMuted
                            }
                        }
                    }
                }
            }
        }
    }
}
