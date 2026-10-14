/*
 * Sun times as a status bar, Windows XP: sunrise, solar noon and sunset in sunken
 * panel cells, the next event in the accent, and the day length (or the sun's
 * elevation when the settings show it) in the last cell. Frameless: the SunTimes
 * window shows it alone, and the Weather window hosts it when both are listed.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format

RowLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property WinStyle ui: WinStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var allEntries: [
        { label: i18ndc(domain, "Sun times: sunrise", "Rise"), epoch: client.sunriseEpochS },
        { label: i18ndc(domain, "Sun times: solar noon", "Noon"), epoch: client.solarNoonEpochS },
        { label: i18ndc(domain, "Sun times: sunset", "Set"), epoch: client.sunsetEpochS }
    ]
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation
    /*! The elevation cell replaces noon: four cells would not fit in 19 grid units. */
    readonly property var entries: showElevation ? [allEntries[0], allEntries[2]] : allEntries
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }

    spacing: root.ui.snap(2)

    /*! A sunken status-bar cell: a small label and its value on the panel colour. */
    component StatusCell: Rectangle {
        id: cell

        required property string label
        required property string value
        property bool highlight: false

        Layout.fillWidth: true
        implicitHeight: root.ui.rowH
        color: root.ui.panel
        border.width: root.ui.line
        border.color: root.ui.etchDark

        Row {
            anchors.centerIn: parent
            spacing: root.ui.gap

            Text {
                anchors.baseline: cellValue.baseline
                text: cell.label
                textFormat: Text.PlainText
                font: root.ui.smallFont
                color: root.ui.inkMuted
            }
            Text {
                id: cellValue

                text: cell.value
                textFormat: Text.PlainText
                font: cell.highlight ? root.ui.boldFont : root.ui.figureFont
                color: cell.highlight ? root.ui.frame : root.ui.ink
            }
        }
    }

    Repeater {
        model: root.entries

        delegate: StatusCell {
            required property var modelData

            label: modelData.label
            value: Format.timeText(modelData.epoch)
            highlight: modelData.epoch === root.nextEpoch
        }
    }

    StatusCell {
        visible: root.showElevation
        label: i18ndc(root.domain, "Sun times: solar elevation", "Elev")
        value: i18ndc(root.domain, "Angle in degrees", "%1\u00B0", root.client.solarElevation.toFixed(1))
    }
}
