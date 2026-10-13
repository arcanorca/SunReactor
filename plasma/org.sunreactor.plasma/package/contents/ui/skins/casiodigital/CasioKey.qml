/*
 * A physical key on the case: a raised, rounded cap in a recess. The chosen key
 * sits pressed in with a backlit LED; `call` marks the way back (the automatic
 * key while something else is chosen); `arrow` adds a small down arrow that opens
 * a menu of durations and has its own hit area.
 *
 * Space and Enter press the key; the arrow is opened by its own click.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    required property CasioStyle style
    property string label: ""
    /*! The chosen key: pressed in, LED lit. */
    property bool lit: false
    /*! The way back: an LED ring instead of a dot. */
    property bool call: false
    property bool arrow: false
    property bool interactive: true

    readonly property bool hovered: hover.containsMouse
    readonly property real arrowSize: style.snap(style.sizeCap * 0.7)
    readonly property real led: style.snap(style.sizeCap * 0.42)

    signal clicked()
    signal arrowClicked()

    implicitHeight: style.snap(style.unit * 2.1)
    implicitWidth: style.snap(style.unit * 4)
    activeFocusOnTab: interactive

    Accessible.role: Accessible.RadioButton
    Accessible.name: label
    Accessible.checkable: true
    Accessible.checked: lit
    Accessible.onPressAction: root.clicked()

    Keys.onPressed: event => {
        if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
            root.clicked();
            event.accepted = true;
        }
    }

    // The recess the key sits in.
    Rectangle {
        anchors.fill: parent
        radius: root.style.keyRadius + 1
        antialiasing: true
        color: root.style.caseBottom
        border.width: root.style.hairline
        border.color: root.style.caseEdge
    }

    // The cap. Pressed in, it moves down a device pixel and reverses its shade.
    Rectangle {
        id: cap

        x: root.style.hairline * 2
        y: root.lit ? root.style.hairline * 3 : root.style.hairline
        width: parent.width - root.style.hairline * 4
        height: parent.height - root.style.hairline * 4
        radius: root.style.keyRadius
        antialiasing: true
        border.width: root.style.hairline
        border.color: root.style.caseEdge
        gradient: Gradient {
            GradientStop { position: 0.0; color: root.lit ? root.style.keyDownTop : root.style.keyTop }
            GradientStop { position: 1.0; color: root.lit ? root.style.keyDownBottom : root.style.keyBottom }
        }
        Rectangle {
            visible: !root.lit
            x: root.style.keyRadius
            y: root.style.hairline
            width: parent.width - 2 * root.style.keyRadius
            height: root.style.hairline
            color: root.style.caseShine
        }
    }

    // The backlight LED: a dot for the chosen key, a ring for the way back.
    Rectangle {
        x: root.width - width - root.style.gap * 1.6
        y: root.style.gap
        width: root.led
        height: root.led
        radius: width / 2
        antialiasing: true
        color: root.lit ? root.style.backlight : "transparent"
        border.width: root.call ? root.style.line2 : 0
        border.color: root.style.backlight
        visible: root.lit || root.call
    }

    Text {
        id: caption

        anchors.centerIn: parent
        anchors.verticalCenterOffset: root.lit ? root.style.hairline * 2 : 0
        text: root.label
        textFormat: Text.PlainText
        font: root.style.cap(root.style.sizeCap * 1.1)
        color: root.lit ? root.style.legend : root.style.legendMuted
        opacity: root.interactive ? 1 : 0.5
    }

    // A small down arrow for the durations, with its own hit area.
    Item {
        id: arrowHit

        visible: root.arrow
        anchors.right: parent.right
        anchors.rightMargin: root.style.gap
        anchors.verticalCenter: parent.verticalCenter
        width: root.arrowSize + root.style.gap * 2
        height: parent.height

        Shape {
            anchors.centerIn: parent
            width: root.arrowSize
            height: root.arrowSize
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                fillColor: root.lit ? root.style.legend : root.style.legendMuted
                PathSvg {
                    path: "M0 0 H" + root.arrowSize + " L" + root.arrowSize / 2 + " " + root.arrowSize * 0.7 + " Z"
                }
            }
        }

        MouseArea {
            anchors.fill: parent
            cursorShape: Qt.PointingHandCursor
            onClicked: root.arrowClicked()
        }
    }

    // Keyboard focus: a backlit ring round the cap.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -root.style.line2
        visible: root.activeFocus
        color: "transparent"
        radius: root.style.keyRadius + 2
        border.width: root.style.line2
        border.color: root.style.backlight
    }

    MouseArea {
        id: hover

        anchors.fill: parent
        anchors.rightMargin: root.arrow ? arrowHit.width : 0
        hoverEnabled: true
        enabled: root.interactive
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.clicked();
        }
    }
}
