/*
 * Automation, cyberpunk: the mode as three chamfered buttons, then what the daemon
 * is doing. The selected one is a solid yellow face; while the daemon is not
 * automatic, AUTOMATIC takes the yellow outline as the way back. Manual and paused
 * raise a hazard alert: the words go red for an override, yellow for a pause.
 *
 *   ░░░ hazard header ░░░
 *   [ AUTOMATIC ] [ MANUAL ] [ PAUSED ▾ ]
 *   Following the sun and the weather · Next 4:20 PM · Set 50% → 23%
 *
 * Left and Right move between buttons, Space or Enter picks one, Down opens the
 * pause durations on PAUSED. The rules are in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property CyberStyle style: CyberStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected
    readonly property bool alert: control.choice === "manual" || control.choice === "paused"
    readonly property color statusTone: control.hinting || control.choice === "manual"
        ? style.red : (control.choice === "paused" ? style.yellow : style.ink)

    spacing: 0

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    function focusCell(index) {
        const count = control.order.length;
        const cell = cells.itemAt((index + count) % count);
        if (cell) {
            cell.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function openDurations(cell) {
        durations.x = Math.max(0, cell.mapToItem(root, 0, 0).x);
        durations.y = cell.mapToItem(root, 0, cell.height).y;
        durations.open();
    }

    CyberFrame {
        Layout.fillWidth: true
        style: root.style
        title: i18ndc(root.control.domain, "Panel title: automation mode", "Automation")

        // The header: a thin band of hazard stripes.
        HazardStrip {
            Layout.fillWidth: true
            style: root.style
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            Repeater {
                id: cells

                model: root.control.choices

                delegate: ChamferButton {
                    id: cell

                    required property var modelData
                    required property int index

                    readonly property string choiceId: modelData.id
                    readonly property bool lit: root.control.choice === choiceId
                    readonly property bool wayBack: choiceId === "automatic" && root.control.choice !== ""
                        && root.control.choice !== "automatic"

                    style: root.style
                    text: modelData.label
                    // The paused arrow takes room at the right: centre the label in what is left.
                    labelOffset: choiceId === "paused" ? -root.style.snap(root.style.unit * 0.9) : 0
                    active: lit
                    call: wayBack
                    font: root.style.labelFont
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    activeFocusOnTab: true

                    Accessible.role: Accessible.RadioButton
                    Accessible.name: modelData.label
                    Accessible.description: modelData.tip
                    Accessible.checkable: true
                    Accessible.checked: lit
                    Accessible.onPressAction: root.control.pick(choiceId, root)

                    Keys.onLeftPressed: root.focusCell(index - 1)
                    Keys.onRightPressed: root.focusCell(index + 1)
                    Keys.onPressed: event => {
                        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                            root.control.pick(choiceId, root);
                            event.accepted = true;
                        } else if (event.key === Qt.Key_Down && choiceId === "paused") {
                            root.openDurations(cell);
                            event.accepted = true;
                        }
                    }

                    onClicked: {
                        cell.forceActiveFocus(Qt.MouseFocusReason);
                        root.control.pick(choiceId, root);
                    }

                    // The arrow has its own hit area: a click there opens the durations.
                    Kirigami.Icon {
                        visible: cell.choiceId === "paused"
                        anchors.right: parent.right
                        anchors.rightMargin: root.style.cut * 2
                        anchors.verticalCenter: parent.verticalCenter
                        width: root.style.snap(root.style.unit * 0.75)
                        height: width
                        source: "arrow-down"
                        isMask: true
                        color: cell.active ? root.style.onYellow : root.style.muted
                    }
                    MouseArea {
                        visible: cell.choiceId === "paused"
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.bottom: parent.bottom
                        width: root.style.snap(root.style.unit * 2)
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.openDurations(cell)
                    }
                }
            }
        }

        // The alert for a manual or paused daemon: hazard stripes under the buttons.
        HazardStrip {
            Layout.fillWidth: true
            visible: root.alert
            height: root.style.snap(root.style.unit * 0.9)
            style: root.style
        }

        Text {
            Layout.fillWidth: true
            text: root.control.displayText
            textFormat: Text.PlainText
            font: root.style.bodyFont
            color: root.statusTone
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.control.note
            textFormat: Text.PlainText
            font: root.style.techFont
            color: root.style.muted
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
