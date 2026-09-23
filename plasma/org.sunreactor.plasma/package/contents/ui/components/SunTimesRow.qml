/*
 * Today's three turning points, with the day drawn as a track underneath so
 * the current moment has a place. The next event is emphasised; the sun's
 * height is only spelled out when the user asked for it.
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

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property var entries: [
        { label: i18nd(root.domain, "Sunrise"), epoch: client.sunriseEpochS },
        { label: i18nd(root.domain, "Solar noon"), epoch: client.solarNoonEpochS },
        { label: i18nd(root.domain, "Sunset"), epoch: client.sunsetEpochS },
    ]
    /*! Epoch of the next event still ahead today, or 0 once the day is done. */
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }
    /*! How far the day has run, 0 at sunrise and 1 at sunset. */
    readonly property real dayProgress: {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (rise <= 0 || set <= rise) {
            return 0;
        }
        return Math.max(0, Math.min(1, (client.nowEpochS - rise) / (set - rise)));
    }
    readonly property bool daylight: client.nowEpochS >= client.sunriseEpochS
        && client.nowEpochS < client.sunsetEpochS
    /*! Set by the popup from the widget's settings; off unless asked for. */
    property bool showSolarElevation: false
    readonly property bool showElevation: showSolarElevation && client.hasSolarElevation

    Layout.fillWidth: true
    visible: client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    background.visible: false
    hoverEnabled: false
    Accessible.ignored: true

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.gridUnit

            Repeater {
                model: root.entries

                delegate: ColumnLayout {
                    id: entry

                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 0

                    PlasmaExtras.DescriptiveLabel {
                        Layout.fillWidth: true
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignHCenter
                    }

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: Format.timeText(entry.modelData.epoch)
                        textFormat: Text.PlainText
                        horizontalAlignment: Text.AlignHCenter
                        font.features: ({ "tnum": 1 })
                        font.weight: entry.modelData.epoch === root.nextEpoch ? Font.Bold : Font.Normal
                    }
                }
            }
        }

        // The day as a track, with the sun itself riding along it. The sprite
        // is the same one the panel shows, at its native 16px grid.
        Item {
            Layout.fillWidth: true
            implicitHeight: 16

            Rectangle {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                height: Math.max(2, Kirigami.Units.smallSpacing / 2)
                radius: height / 2
                color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                               Kirigami.Theme.textColor.b, 0.15)

                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: parent.width * root.dayProgress
                    radius: parent.radius
                    color: Qt.rgba(Kirigami.Theme.highlightColor.r, Kirigami.Theme.highlightColor.g,
                                   Kirigami.Theme.highlightColor.b, root.daylight ? 0.6 : 0.25)
                }
            }

            Kirigami.Icon {
                width: 16
                height: 16
                x: (parent.width - width) * root.dayProgress
                anchors.verticalCenter: parent.verticalCenter
                roundToIconSize: false
                source: Icons.skyArt(!root.daylight)

                Behavior on x {
                    NumberAnimation {
                        duration: Kirigami.Units.longDuration
                        easing.type: Easing.InOutQuad
                    }
                }
            }
        }

        PlasmaExtras.DescriptiveLabel {
            Layout.fillWidth: true
            visible: root.showElevation
            horizontalAlignment: Text.AlignHCenter
            textFormat: Text.PlainText
            text: root.client.solarElevation >= 0
                ? i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° above the horizon", root.client.solarElevation.toFixed(1))
                : i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° below the horizon", (-root.client.solarElevation).toFixed(1))
        }
    }
}
