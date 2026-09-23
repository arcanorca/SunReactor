/*
 * The next few forecast intervals the daemon already fetched for its own
 * policy. Same sprites as the panel, so the strip reads at a glance.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../Format.js" as Format
import "../Icons.js" as Icons

PlasmaComponents3.ItemDelegate {
    id: root

    required property SunReactorClient client
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

    Layout.fillWidth: true
    visible: points.length > 0

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

                PlasmaExtras.DescriptiveLabel {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: Format.timeText(slot.modelData.epochS)
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                }

                Kirigami.Icon {
                    Layout.alignment: Qt.AlignHCenter
                    // Two screen pixels per art pixel.
                    Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                    Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                    roundToIconSize: false
                    source: Icons.conditionArt(slot.modelData.condition, slot.modelData.isNight)
                }

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: i18ndc("plasma_applet_org.sunreactor.plasma",
                                 "Temperature in degrees, compact", "%1°",
                                 Math.round(slot.modelData.temperatureC))
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                }
            }
        }
    }
}
