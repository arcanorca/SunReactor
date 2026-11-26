/*
 * Xbox 360 Sun Times: metallic blade bar showing the daylight arc across 24h
 * with green/amber solar markers and Chakra Petch clock times.
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
    readonly property XboxStyle style: XboxStyle {
        theme: root.theme
    }
    readonly property bool available: (client.isConnected || client.statusStale) && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    readonly property real dayStart: {
        const d = new Date(client.sunriseEpochS * 1000);
        d.setHours(0, 0, 0, 0);
        return d.getTime() / 1000;
    }
    readonly property real daySpan: 86400
    function fractionOf(epoch) {
        return Math.max(0, Math.min(1, (epoch - dayStart) / daySpan));
    }

    readonly property real riseF: fractionOf(client.sunriseEpochS)
    readonly property real setF: fractionOf(client.sunsetEpochS)
    readonly property real noonF: client.solarNoonEpochS > 0 ? fractionOf(client.solarNoonEpochS) : (riseF + setF) / 2
    readonly property real nowF: fractionOf(client.nowEpochS)
    readonly property bool nowInDay: client.nowEpochS >= dayStart && client.nowEpochS <= dayStart + daySpan

    spacing: 0

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(domain, "Title of the sun times panel", "Sun")

    XboxCard {
        id: card

        style: root.style
        pad: root.style.cardPad
        Layout.fillWidth: true
        implicitHeight: col.implicitHeight + 2 * card.pad

        ColumnLayout {
            id: col

            anchors.fill: parent
            spacing: root.style.gap

            Item {
                id: track

                Layout.fillWidth: true
                implicitHeight: root.style.snap(root.style.unit * 0.9)

                readonly property real barH: root.style.snap(root.style.unit * 0.4)
                readonly property real markerD: root.style.snap(root.style.unit * 0.6)

                Rectangle {
                    y: (track.height - track.barH) / 2
                    width: track.width
                    height: track.barH
                    radius: height / 2
                    color: root.style.grooveFill
                    border.width: 1
                    border.color: root.style.grooveEdge
                }
                Rectangle {
                    x: root.riseF * track.width
                    y: (track.height - track.barH) / 2
                    width: Math.max(0, (root.setF - root.riseF) * track.width)
                    height: track.barH
                    radius: height / 2
                    gradient: Gradient {
                        GradientStop { position: 0; color: root.style.grooveGlowTop }
                        GradientStop { position: 0.5; color: root.style.green }
                        GradientStop { position: 1; color: root.style.grooveGlowBottom }
                    }
                }
                Repeater {
                    model: [root.riseF, root.noonF, root.setF]

                    delegate: Rectangle {
                        required property real modelData

                        x: modelData * track.width - width / 2
                        y: (track.height - height) / 2
                        width: track.markerD
                        height: width
                        radius: width / 2
                        color: root.style.warning
                        border.width: 1
                        border.color: root.style.sunMarkerEdge
                    }
                }
                Rectangle {
                    visible: root.nowInDay
                    x: root.nowF * track.width - width / 2
                    y: (track.height - height) / 2
                    width: track.markerD
                    height: width
                    radius: width / 2
                    color: root.style.ink
                    border.width: root.style.line2
                    border.color: root.style.green
                }
            }

            RowLayout {
                Layout.fillWidth: true

                Text {
                    text: i18ndc(root.domain, "Sunrise, compact", "Rise") + " " + Format.timeText(root.client.sunriseEpochS)
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.ink
                }
                Item { Layout.fillWidth: true }
                Text {
                    visible: root.client.solarNoonEpochS > 0
                    text: i18ndc(root.domain, "Solar noon, compact", "Noon") + " " + Format.timeText(root.client.solarNoonEpochS)
                    textFormat: Text.PlainText
                    font: root.style.captionFont
                    color: root.style.muted
                }
                Item { Layout.fillWidth: true }
                Text {
                    text: i18ndc(root.domain, "Sunset, compact", "Set") + " " + Format.timeText(root.client.sunsetEpochS)
                    textFormat: Text.PlainText
                    font: root.style.captionBoldFont
                    color: root.style.ink
                }
            }
        }
    }
}
