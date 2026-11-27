/*
 * Forecast, Nothing: the next intervals the daemon already fetched, one column
 * each: the time in small mono, the pictogram as bare dots, the temperature in
 * the 5x7 dot face. No boxes; a hairline between two intervals is the only rule.
 *
 *   16:00      17:00      18:00      19:00      20:00
 *   [icon]     [icon]     [icon]     [icon]     [icon]
 *   14°        13°        12°        11°        10°
 *
 * Five columns at the narrowest popup, six when the popup is wider.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../Format.js" as Format
import "../../modules/ForecastRules.js" as ForecastRules

Rectangle {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    /*! Intervals across: five, or six once each column has room for it. */
    readonly property int slots: Math.max(5, Math.min(6, Math.floor((root.width - 2 * style.pad) / (root.style.unit * 3.8))))

    readonly property var points: ForecastRules.upcoming(root.client.forecast, root.client.nowEpochS, root.slots)

    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled && points.length > 0

    implicitHeight: row.implicitHeight + 2 * style.pad
    radius: style.radius
    antialiasing: true
    color: style.surface
    border.width: style.hairline
    border.color: style.outline

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(root.settings.domain, "Forecast")

    RowLayout {
        id: row

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: 0

        Repeater {
            model: root.points

            delegate: Item {
                id: slot

                required property var modelData
                required property int index

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                implicitHeight: column.implicitHeight

                // A hairline between two intervals, never at the ends.
                Rectangle {
                    visible: slot.index > 0
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: root.style.hairline
                    color: root.style.outline
                }

                ColumnLayout {
                    id: column

                    anchors.left: parent.left
                    anchors.right: parent.right
                    spacing: root.style.gap * 0.5

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: Format.timeText(slot.modelData.epochS)
                        textFormat: Text.PlainText
                        font: root.style.label
                        color: root.style.muted
                    }

                    ThemedWeatherIcon {
                        Layout.alignment: Qt.AlignHCenter
                        theme: root.theme
                        size: root.style.snap(root.style.unit * 1.9)
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight

                        NothingIcon {
                            anchors.centerIn: parent
                            theme: root.theme
                            size: root.style.snap(root.style.unit * 1.9)
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight
                        }
                    }

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                     Format.tempRounded(slot.modelData.temperatureC, root.client.isFahrenheit))
                        textFormat: Text.PlainText
                        font: root.style.dotValue
                        color: root.style.ink
                    }
                }
            }
        }
    }
}
