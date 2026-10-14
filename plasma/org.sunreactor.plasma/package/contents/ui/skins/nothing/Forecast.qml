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

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    /*! Intervals across: five, or six once each column has room for it. */
    readonly property int slots: Math.max(5, Math.min(6, Math.floor(root.width / (root.style.unit * 4))))

    readonly property var points: {
        const upcoming = [];
        for (const point of root.client.forecast) {
            if (point.epochS > root.client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === root.slots) {
                break;
            }
        }
        return upcoming;
    }

    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    implicitHeight: row.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(root.settings.domain, "Forecast")

    RowLayout {
        id: row

        width: root.width
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
                        size: root.style.pitchFine * 12
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight

                        DotIcon {
                            anchors.centerIn: parent
                            pitch: root.style.pitchFine
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight
                            litColor: root.style.ink
                            accentColor: root.style.accent
                            dimColor: root.style.textGhost
                        }
                    }

                    DotText {
                        Layout.alignment: Qt.AlignHCenter
                        text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                     Math.round(slot.modelData.temperatureC))
                        face: "small"
                        size: 1
                        letterSpacing: 1
                        pitch: root.style.pitchDigit
                        dotScale: root.style.digitScale
                        litColor: root.style.ink
                        accentColor: root.style.accent
                        dimColor: root.style.textGhost
                    }
                }
            }
        }
    }
}
