/*
 * Forecast module: the next few intervals the daemon already fetched for its
 * own policy. Same sprites as the panel, so the strip reads at a glance.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../components"
import "../Format.js" as Format

PlasmaComponents3.ItemDelegate {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    /*! How many intervals to show; five fits the popup at its narrowest. */
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

    Layout.fillWidth: true

    background.visible: false
    hoverEnabled: false
    Accessible.ignored: true

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: root.points

            delegate: ColumnLayout {
                id: slot

                required property var modelData

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: 0

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: Format.timeText(slot.modelData.epochS)
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                    font.family: root.theme.digitsFont
                    font.pointSize: Kirigami.Theme.smallFont.pointSize
                    color: root.theme.textMuted
                    elide: Text.ElideRight
                }

                WeatherIcon {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                    Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                    theme: root.theme
                    condition: slot.modelData.condition
                    night: slot.modelData.isNight
                }

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: i18ndc(root.settings.domain,
                                 "Temperature in degrees, compact", "%1°",
                                 Math.round(slot.modelData.temperatureC))
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                    font.family: root.theme.digitsFont
                    color: root.theme.fg
                }
            }
        }
    }
}
