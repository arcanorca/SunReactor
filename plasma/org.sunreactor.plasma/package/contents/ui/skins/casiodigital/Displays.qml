/*
 * Displays, casiodigital: one row per display on the glass. Each row is the
 * display's name printed in caps, a state flag when it is overridden, off or not
 * answering, the percentage in seven segments, and a 20-cell bar to set it.
 * An "All displays" bar leads when there is more than one display.
 *
 * Bars write overrides, never on scroll; a click on a flag of a display that was
 * set by hand gives it back to automation.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../instrument/Curve.js" as Curve
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
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAllDisplays: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    implicitWidth: bezel.implicitWidth
    implicitHeight: bezel.implicitHeight

    /*! One display (or the master) as a row: caption, flag, figure, bar. */
    component ChannelRow: ColumnLayout {
        id: channel

        property string name: ""
        property int percent: -1
        property int minPct: -1
        property int maxPct: -1
        property int targetPct: -1
        property bool controllable: true
        property bool hasOverride: false
        property string stateText: ""

        signal requested(int percent)
        signal clearRequested()

        Layout.fillWidth: true
        spacing: root.style.gap

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            Text {
                Layout.fillWidth: true
                text: channel.name
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeCap * 1.15)
                color: root.style.ink
                elide: Text.ElideRight
            }

            // A flag that says why: set by hand (click to give it back), off, or silent.
            Item {
                visible: channel.stateText.length > 0
                implicitWidth: stateFlag.implicitWidth
                implicitHeight: stateFlag.implicitHeight

                CasioFlag {
                    id: stateFlag

                    style: root.style
                    text: channel.stateText
                    lit: true
                }
                MouseArea {
                    anchors.fill: parent
                    enabled: channel.hasOverride
                    cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
                    onClicked: channel.clearRequested()
                }
            }

            CasioDigits {
                style: root.style
                size: root.style.digitMedium
                text: Casio.numberText(channel.percent, 3)
                Accessible.name: channel.name + ": " + (channel.percent >= 0 ? channel.percent + "%" : "")
            }
            Text {
                Layout.alignment: Qt.AlignBottom
                text: "%"
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeBody)
                color: root.style.inkMuted
            }
        }

        CasioBar {
            Layout.fillWidth: true
            style: root.style
            value: channel.percent
            minPct: channel.minPct
            maxPct: channel.maxPct
            targetPct: channel.targetPct
            interactive: channel.controllable
            accessibleName: channel.name
            onCommitted: value => channel.requested(value)
        }
    }

    CasioBezel {
        id: bezel

        anchors.left: parent.left
        anchors.right: parent.right
        style: root.style
        title: i18nd(root.domain, "Displays")
        trailing: root.monitors.length > 1
            ? i18ndc(root.domain, "Number of display channels", "%1 CH", root.monitors.length) : ""

        ChannelRow {
            visible: root.showAllDisplays
            name: i18nd(root.domain, "All displays")
            percent: root.client.globalPercent
            stateText: root.client.isOverrideActive
                ? i18ndc(root.domain, "Tag: this brightness was set by hand", "MAN") : ""
            onRequested: value => root.client.setGlobalOverride(value, root.settings.overrideMinutes)
        }

        Repeater {
            model: root.monitors

            delegate: ChannelRow {
                id: display

                required property var modelData

                readonly property bool unavailable: modelData.unreachable === true
                    || modelData.topology === "temporarily_unavailable"

                name: modelData.logicalId
                percent: modelData.percent !== undefined ? modelData.percent : -1
                minPct: modelData.minPct !== undefined ? modelData.minPct : -1
                maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
                targetPct: Math.round(Curve.targetAt(root.client.dayCurve, modelData.logicalId, root.client.nowEpochS))
                controllable: modelData.enabled === true && !unavailable
                hasOverride: modelData.hasOverride === true
                stateText: {
                    if (modelData.enabled === false) {
                        return i18ndc(root.domain, "Tag: this display is excluded from automation", "OFF");
                    }
                    if (unavailable) {
                        return i18ndc(root.domain, "Tag: this display is not answering", "NO LINK");
                    }
                    if (modelData.hasOverride === true) {
                        return i18ndc(root.domain, "Tag: this brightness was set by hand", "MAN");
                    }
                    return "";
                }

                onRequested: value => root.client.setMonitorOverride(modelData.logicalId, value,
                                                                     root.settings.overrideMinutes)
                onClearRequested: root.client.clearMonitorOverride(modelData.logicalId)
            }
        }
    }
}
