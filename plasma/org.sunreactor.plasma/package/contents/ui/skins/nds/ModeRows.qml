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

    // The speech bubble: crisp pixel DS message box with stepped pixel pointer.
    Item {
        id: bubble

        x: 0
        y: root.buttonsH + root.bubbleGap
        width: root.width
        height: root.bubbleH

        BorderImage {
            id: body

            anchors.fill: parent
            source: Qt.resolvedUrl("assets/ds_bubble.png")
            border { left: 4; top: 4; right: 4; bottom: 4 }
            horizontalTileMode: BorderImage.Stretch
            verticalTileMode: BorderImage.Stretch
            smooth: false
        }

        // Crisp 2-step pixel pointer aligned under the Auto button
        Item {
            x: Math.round(root.gu * 1.4)
            y: -4
            width: 8
            height: 5
            Rectangle { x: 2; y: 0; width: 4; height: 2; color: "#7a6430" }
            Rectangle { x: 0; y: 2; width: 8; height: 2; color: "#7a6430" }
            Rectangle { x: 3; y: 1; width: 2; height: 1; color: root.style.cream }
            Rectangle { x: 1; y: 2; width: 6; height: 3; color: root.style.cream }
        }

        Column {
            id: sentence

            readonly property string subLine: root.control.joinParts([root.control.scheduleText, root.control.note])

            anchors.verticalCenter: body.verticalCenter
            anchors.left: body.left
            anchors.right: body.right
            anchors.leftMargin: Math.round(root.gu * 0.45)
            anchors.rightMargin: Math.round(root.gu * 0.45)
            spacing: Math.round(root.gu * 0.14)

            Accessible.role: Accessible.StaticText
            Accessible.name: root.control.statusLine

            Text {
                width: parent.width
                text: root.control.primaryText
                textFormat: Text.PlainText
                font.family: root.style.family
                font.pixelSize: Math.round(root.gu * 0.56)
                color: root.control.hinting ? root.style.blue : root.style.ink
                elide: Text.ElideRight
            }

            Text {
                width: parent.width
                visible: text.length > 0
                text: sentence.subLine
                textFormat: Text.PlainText
                font.family: root.style.family
                font.pixelSize: Math.round(root.gu * 0.50)
                color: root.style.muted
                elide: Text.ElideRight
            }
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
                durations.openBelow(cell, root);
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

        // Crisp pixel selection indicator on the lit DS button
        Rectangle {
            visible: cell.lit
            x: cell.width - Math.round(root.gu * 0.95)
            y: Math.round(root.gu * 0.38)
            width: 4
            height: 4
            radius: 0
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
                onClicked: durations.openBelow(cell, root)
            }
        }
    }
}
