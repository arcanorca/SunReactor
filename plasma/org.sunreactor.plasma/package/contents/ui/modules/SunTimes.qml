/*
 * Sun times module: today's three turning points, with the day drawn as a
 * track underneath so the current moment has a place. The next event is
 * emphasised; the sky sprite rides the track at its native 16 px. The sun's
 * height is only spelled out when the user asked for it.
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

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
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
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    Layout.fillWidth: true

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
                    readonly property bool isNext: modelData.epoch === root.nextEpoch

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 0

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        font.family: root.theme.bodyFont
                        color: root.theme.textMuted
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignHCenter
                    }

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: Format.timeText(entry.modelData.epoch)
                        textFormat: Text.PlainText
                        horizontalAlignment: Text.AlignHCenter
                        font.features: ({ "tnum": 1 })
                        font.family: root.theme.digitsFont
                        font.weight: entry.isNext ? Font.Bold : Font.Normal
                        color: entry.isNext ? root.theme.accent : root.theme.fg
                    }
                }
            }
        }

        // The day track: sunrise at the left end, sunset at the right.
        Item {
            id: track

            Layout.fillWidth: true
            implicitHeight: 16

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width
                height: root.theme.lineWidth * 2
                radius: root.theme.isVector ? height / 2 : 0
                color: root.theme.line
            }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width * root.dayProgress
                height: root.theme.lineWidth * 2
                radius: root.theme.isVector ? height / 2 : 0
                color: root.daylight ? root.theme.accent : root.theme.textMuted
            }

            WeatherIcon {
                // The sprite's native 16 px, on a whole logical pixel.
                size: 16
                x: Math.round(root.dayProgress * (track.width - width))
                anchors.verticalCenter: parent.verticalCenter
                theme: root.theme
                condition: "clear"
                night: !root.daylight

                Behavior on x {
                    NumberAnimation { duration: Kirigami.Units.longDuration }
                }
            }
        }

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            visible: root.showElevation
            horizontalAlignment: Text.AlignHCenter
            textFormat: Text.PlainText
            font.family: root.theme.bodyFont
            font.pointSize: Kirigami.Theme.smallFont.pointSize
            color: root.theme.textMuted
            text: root.client.solarElevation >= 0
                ? i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° above the horizon", root.client.solarElevation.toFixed(1))
                : i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° below the horizon", (-root.client.solarElevation).toFixed(1))
        }
    }
}
