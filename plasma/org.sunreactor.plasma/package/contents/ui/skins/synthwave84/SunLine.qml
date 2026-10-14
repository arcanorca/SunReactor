/*
 * The sun times on one line, synthwave84, without a card: sunrise, solar noon and
 * sunset, each a caps legend beside its time. The next one is in pink. Framed by SunTimes.qml's card, or hosted by the Weather card.
 *
 *   RISE 7:13 AM   NOON 12:55 PM   SET 6:38 PM
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules" as Mod
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property Mod.ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property SynthStyle style: SynthStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var events: [
        { label: i18ndc(domain, "Short legend: sunrise", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Short legend: solar noon", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Short legend: sunset", "Set"), epoch: client.sunsetEpochS }
    ]
    /*! Index of the next event, or -1 once the day has gone by. */
    readonly property int nextIndex: {
        for (let i = 0; i < events.length; ++i) {
            if (events[i].epoch > client.nowEpochS) {
                return i;
            }
        }
        return -1;
    }

    spacing: 0

    Accessible.role: Accessible.StaticText
    Accessible.name: available
        ? i18nd(domain, "Sun times") + ": " + events.map(e => e.label + " " + Format.timeText(e.epoch)).join(", ")
        : ""

    Flow {
        Layout.fillWidth: true
        spacing: root.style.gap2

        Repeater {
            model: root.available ? root.events : []

            delegate: Row {
                id: event

                required property var modelData
                required property int index

                spacing: root.style.gap

                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: event.modelData.label
                    textFormat: Text.PlainText
                    font: root.style.labelFont
                    color: event.index === root.nextIndex ? root.style.pink : root.style.cyan
                }
                Text {
                    anchors.verticalCenter: parent.verticalCenter
                    text: Format.timeText(event.modelData.epoch)
                    textFormat: Text.PlainText
                    font: root.style.dataSmallFont
                    color: root.style.ink
                }
            }
        }
    }
}
