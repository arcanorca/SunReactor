/*
 * Forecast, casiodigital: the next five hours as five readouts in a row, each the
 * hour in print, the condition as a solid glyph in the ink, and the temperature
 * in seven segments.
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

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Forecast")
        trailing: i18ndc(root.domain, "Forecast horizon in hours", "%1 H", root.slots)

        RowLayout {
            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: root.points

                delegate: ColumnLayout {
                    id: slot

                    required property var modelData
                    required property int index

                    // The hour only ("3 PM", or "15" in a 24-hour locale): five of them must fit.
                    readonly property var clock: Casio.clockParts(Format.timeText(modelData.epochS))
                    readonly property string hourText: clock.digits.split(":")[0].trim()
                        + (clock.suffix.length > 0 ? " " + clock.suffix : "")

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
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
                        size: root.style.snap(root.style.unit * 1.5)
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight === true
                        Layout.alignment: Qt.AlignHCenter
                        WeatherGlyph {
                            anchors.centerIn: parent
                            size: root.style.snap(root.style.unit * 1.5)
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight === true
                            glyphStyle: "solid"
                            palette: root.style.glyphPalette
                        }
                    }

                    RowLayout {
                        Layout.alignment: Qt.AlignHCenter
                        spacing: root.style.snap(root.style.gap * 0.5)

                        CasioDigits {
                            style: root.style
                            size: root.style.digitSmall
                            text: Casio.numberText(slot.modelData.temperatureC, 2)
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
