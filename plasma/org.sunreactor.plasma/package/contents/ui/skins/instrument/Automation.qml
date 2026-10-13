/*
 * Automation, instrument family: the mode as a three-cell front-panel switch,
 * then what the daemon is doing and until when.
 *
 *   AUTOMATION ──────────────────────────────────────────────
 *   ┌──────────────┬──────────────┬──────────────┐
 *   │  AUTOMATIC   │    MANUAL    │  PAUSED  ▾   │
 *   └──────────────┴──────────────┴──────────────┘
 *   Following the sun and the weather · Next 4:20 PM · Set 50% → 23%
 *
 * The lit cell is the state (ok for automatic, warning for manual and paused).
 * While the daemon is not automatic, AUTOMATIC takes the accent: the way back.
 * Left and Right move between cells, Space or Enter picks one, Down opens the
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

    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected

    spacing: style.rowGap

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

    Legend {
        Layout.fillWidth: true
        style: root.style
        text: i18ndc(root.control.domain, "Instrument legend over the automation state", "Automation")
    }

    Rectangle {
        id: panel

        Layout.fillWidth: true
        implicitHeight: root.style.snap(root.style.sizeLegend * 2.5)
        radius: root.style.tagRadius
        antialiasing: radius > 0
        color: "transparent"

        RowLayout {
            anchors.fill: parent
            spacing: 0

            Repeater {
                id: cells

                model: root.control.choices

                delegate: Item {
                    id: cell

                    required property var modelData
                    required property int index

                    readonly property string choiceId: modelData.id
                    readonly property bool lit: root.control.choice === choiceId
                    readonly property bool call: choiceId === "automatic" && root.control.choice !== ""
                        && root.control.choice !== "automatic"
                    readonly property color tone: choiceId === "automatic" ? root.style.ok : root.style.warning
                    readonly property color ink: {
                        if (lit) {
                            return root.style.theme.onColor(tone);
                        }
                        if (call) {
                            return root.style.accent;
                        }
                        return hover.containsMouse ? root.style.ink : root.style.inkMuted;
                    }

                    Layout.fillWidth: true
                    Layout.fillHeight: true
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

                    // Lit cell face, the accent frame of the way back, hairline divider, focus ring.
                    Rectangle {
                        anchors.fill: parent
                        color: cell.lit ? cell.tone : "transparent"
                    }
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: root.style.line2
                        visible: cell.call
                        color: "transparent"
                        border.width: root.style.line2
                        border.color: root.style.accent
                    }
                    Rectangle {
                        anchors.fill: parent
                        visible: !cell.lit && !cell.call && hover.containsMouse
                        color: root.style.theme.alpha(root.style.ink, 0.08)
                    }
                    Rectangle {
                        visible: cell.index > 0
                        anchors.left: parent.left
                        width: root.style.hairline
                        height: parent.height
                        color: root.style.ruleStrong
                    }
                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: root.style.line2
                        visible: cell.activeFocus
                        color: "transparent"
                        border.width: root.style.line2
                        border.color: root.style.theme.borderActive
                    }

                    Row {
                        anchors.centerIn: parent
                        spacing: root.style.gap

                        Text {
                            anchors.verticalCenter: parent.verticalCenter
                            text: cell.modelData.label
                            textFormat: Text.PlainText
                            font: root.style.legendFont
                            color: cell.ink
                        }

                        // The arrow has its own hit area: a click there opens the durations.
                        Item {
                            visible: cell.choiceId === "paused"
                            anchors.verticalCenter: parent.verticalCenter
                            width: arrow.width + root.style.gap * 2
                            height: root.style.snap(root.style.sizeLegend * 2)

                            Kirigami.Icon {
                                id: arrow

                                anchors.centerIn: parent
                                width: root.style.snap(root.style.sizeLegend)
                                height: width
                                source: "arrow-down"
                                isMask: true
                                color: cell.ink
                            }
                            MouseArea {
                                anchors.fill: parent
                                cursorShape: Qt.PointingHandCursor
                                onClicked: root.openDurations(cell)
                            }
                        }
                    }

                    MouseArea {
                        id: hover

                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            cell.forceActiveFocus(Qt.MouseFocusReason);
                            root.control.pick(cell.choiceId, root);
                        }
                    }
                }
            }
        }

        // The frame is drawn over the cells, so a lit cell does not cover its edge.
        Rectangle {
            anchors.fill: parent
            color: "transparent"
            radius: root.style.tagRadius
            antialiasing: radius > 0
            border.width: root.style.hairline
            border.color: root.style.ruleStrong
        }
    }

    Text {
        Layout.fillWidth: true
        text: root.control.displayText
        textFormat: Text.PlainText
        font: root.style.bodyFont
        color: root.control.hinting ? root.style.warning : root.style.ink
        wrapMode: Text.WordWrap
        maximumLineCount: 2
        elide: Text.ElideRight
    }

    Text {
        Layout.fillWidth: true
        visible: text.length > 0
        text: root.control.note
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.style.inkMuted
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
