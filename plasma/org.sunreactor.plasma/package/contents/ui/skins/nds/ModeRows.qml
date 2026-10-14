/*
 * The automation controls of the Nintendo DS touch screen: AUTO, MANUAL and PAUSE
 * as three big touch buttons, and the daemon's sentence under them in a small cream
 * speech bubble. The chosen one is filled DS blue with white type and a stylus-dot
 * highlight where the pen last touched it. PAUSE has a chevron that opens the pause
 * durations. Displays.qml draws this inside the bottom screen; Automation.qml draws
 * it alone when the theme has no displays. The rules are in modules/ModeControl.qml.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings
    required property NdsStyle style

    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }

    readonly property real gu: style.gu
    readonly property real gap: Math.round(gu * 0.45)
    /*! The button row, a gap for the bubble's tail, then the bubble: two lines of type. */
    readonly property real buttonsH: Math.round(gu * 1.9)
    readonly property real bubbleGap: Math.round(gu * 0.45)
    readonly property real bubbleH: Math.round(gu * 2.2)
    /*! The whole tray: what the host screen reserves for it. */
    readonly property real trayH: buttonsH + bubbleGap + bubbleH

    /*! The button captions: the short forms fit three big touch buttons. */
    readonly property var captions: ({
        automatic: i18ndc(control.domain, "Mode segment, compact: follow the sun again", "Auto"),
        manual: i18ndc(control.domain, "Mode segment, compact: set brightness by hand", "Manual"),
        paused: i18ndc(control.domain, "Mode segment, compact: stop adjusting for a while", "Pause"),
    })

    /*! The status sentence and the idle note, joined: one line that wraps. */
    readonly property string statusLine: control.hinting || control.note.length === 0
        ? control.displayText
        : control.displayText + " · " + control.note

    function openDurations(cell) {
        durations.x = Math.max(0, cell.mapToItem(root, 0, 0).x);
        durations.y = cell.mapToItem(root, 0, cell.height).y;
        durations.open();
    }

    implicitHeight: trayH

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    Row {
        id: buttons

        width: parent.width
        height: root.buttonsH
        spacing: root.gap

        Repeater {
            model: root.control.choices

            delegate: ModeButton {
                width: Math.floor((buttons.width - 2 * root.gap) / 3)
                height: buttons.height
            }
        }
    }

    // The speech bubble: cream, with a small tail that points up at the buttons.
    Item {
        id: bubble

        x: 0
        y: root.buttonsH + root.bubbleGap
        width: root.width
        height: root.bubbleH

        // The tail: a cream point on the bubble's top edge, outlined like the body.
        Shape {
            id: tail

            x: Math.round(root.gu * 1.2)
            y: -height
            width: Math.round(root.gu * 0.8)
            height: Math.round(root.gu * 0.4)

            ShapePath {
                strokeWidth: 1
                strokeColor: root.style.creamEdge
                fillColor: root.style.cream
                startX: 0
                startY: tail.height
                PathLine { x: tail.width / 2; y: 0 }
                PathLine { x: tail.width; y: tail.height }
            }
        }

        Rectangle {
            id: body

            anchors.fill: parent
            radius: Math.round(root.gu * 0.6)
            color: root.style.cream
            border.width: 1
            border.color: root.style.creamEdge
        }

        Text {
            id: sentence

            anchors.fill: body
            anchors.margins: Math.round(root.gu * 0.15)
            verticalAlignment: Text.AlignVCenter
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
            text: root.statusLine
            font.family: root.style.family
            font.pixelSize: Math.round(root.gu * 0.62)
            color: root.control.hinting ? root.style.blue : root.style.ink
            Accessible.role: Accessible.StaticText
            Accessible.name: root.statusLine
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }

    /*! One touch button: a DS panel, filled blue when it is the chosen mode. */
    component ModeButton: NdsPanel {
        id: cell

        required property var modelData

        readonly property string choiceId: modelData.id
        readonly property bool lit: root.control.choice === choiceId

        style: root.style
        kind: lit ? "ds" : "panel"
        outlineColor: root.style.blue

        Accessible.role: Accessible.RadioButton
        Accessible.name: modelData.label
        Accessible.description: modelData.tip
        Accessible.checkable: true
        Accessible.checked: lit
        Accessible.onPressAction: root.control.pick(choiceId, cell)
        activeFocusOnTab: true

        Keys.onPressed: event => {
            if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                root.control.pick(choiceId, cell);
                event.accepted = true;
            } else if (event.key === Qt.Key_Down && choiceId === "paused") {
                root.openDurations(cell);
                event.accepted = true;
            }
        }

        Text {
            anchors.centerIn: parent
            anchors.horizontalCenterOffset: cell.choiceId === "paused" ? -Math.round(root.gu * 0.3) : 0
            text: root.captions[cell.choiceId] || cell.modelData.label
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.0)
            color: cell.lit ? root.style.white : root.style.blue
        }

        // The stylus dot: a white touch point with a soft halo, on the lit button.
        Rectangle {
            visible: cell.lit
            x: cell.width - Math.round(root.gu * 1.25)
            y: Math.round(root.gu * 0.3)
            width: Math.round(root.gu * 0.8)
            height: width
            radius: width / 2
            color: Qt.rgba(1, 1, 1, 0.22)
        }
        Rectangle {
            visible: cell.lit
            x: cell.width - Math.round(root.gu * 1.05)
            y: Math.round(root.gu * 0.5)
            width: Math.round(root.gu * 0.36)
            height: width
            radius: width / 2
            color: root.style.white
        }

        // The whole button is the touch target; the chevron below takes its own clicks.
        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                cell.forceActiveFocus(Qt.MouseFocusReason);
                root.control.pick(cell.choiceId, cell);
            }
        }

        // The chevron of PAUSE: a click or a Down key opens the durations.
        Item {
            visible: cell.choiceId === "paused"
            x: cell.width - Math.round(root.gu * 1.2)
            y: Math.round((cell.height - height) / 2)
            width: Math.round(root.gu * 1.0)
            height: Math.round(root.gu * 1.0)

            Kirigami.Icon {
                anchors.centerIn: parent
                width: Math.round(root.gu * 0.7)
                height: width
                source: "arrow-down"
                isMask: true
                color: cell.lit ? root.style.white : root.style.blue
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.openDurations(cell)
            }
        }
    }
}
