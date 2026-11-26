/*
 * An XP trackbar: a sunken groove filled with the green blocks of XP's progress
 * bar up to the value, and the pentagon pointer at the value. A press or a drag
 * sets the value; `requested` comes at most every 160 ms while dragging and once
 * on release. Left and Right move it by 5.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "../../components"

Item {
    id: root

    required property WinStyle style
    /*! 0..100. */
    property int value: 0
    property int from: 0
    property int to: 100
    property bool controllable: true

    readonly property real pitch: 10
    readonly property real block: 8
    readonly property real thumbW: root.style.snap(root.style.sizeBody * 0.9)
    readonly property real thumbH: root.style.snap(root.style.sizeBody * 1.45)
    readonly property real grooveH: root.style.snap(root.style.sizeBody * 0.8)
    /*! Distance the thumb's centre travels. */
    readonly property real travel: Math.max(0, width - thumbW)
    /*! -1 while the pointer is not dragging; otherwise the value under it. */
    property int dragValue: -1
    readonly property int shown: dragValue >= 0 ? dragValue
        : (throttle.pending ? throttle.pendingValue : Math.max(from, Math.min(to, value)))
    readonly property real fillLength: shown / 100 * travel

    signal requested(int value)

    implicitWidth: thumbW * 6
    implicitHeight: thumbH
    opacity: controllable ? 1 : 0.45
    activeFocusOnTab: controllable

    Accessible.role: Accessible.Slider
    Accessible.name: i18nd("plasma_applet_org.sunreactor.plasma", "Brightness")

    onValueChanged: {
        if (!pointer.pressed && !throttle.pending) {
            dragValue = -1;
        }
    }

    // The groove and the blocks.
    Rectangle {
        id: groove

        x: root.thumbW / 2
        y: (root.height - root.grooveH) / 2
        width: root.travel
        height: root.grooveH
        radius: 2
        color: root.style.trough
        border.width: root.style.line
        border.color: root.style.etchDark
    }

    Repeater {
        model: Math.ceil(root.travel / root.pitch)

        delegate: Rectangle {
            required property int index

            x: groove.x + root.style.line + index * root.pitch
            y: groove.y + root.style.line
            width: root.block
            height: groove.height - 2 * root.style.line
            visible: index * root.pitch + root.block <= root.fillLength
            color: root.style.green
            gradient: Gradient {
                GradientStop { position: 0; color: root.style.greenSheen }
                GradientStop { position: 0.45; color: root.style.green }
                GradientStop { position: 1; color: root.style.green }
            }
        }
    }

    // The pentagon pointer, its point down.
    Shape {
        id: thumb

        x: root.fillLength
        y: (root.height - root.thumbH) / 2
        width: root.thumbW
        height: root.thumbH
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.style.frameDeep
            strokeWidth: root.style.line
            joinStyle: ShapePath.MiterJoin
            startX: 0.5
            startY: 0.5
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: 0
                y2: thumb.height
                GradientStop { position: 0; color: root.style.white }
                GradientStop { position: 1; color: root.style.panel }
            }
            PathLine { x: thumb.width - 0.5; y: 0.5 }
            PathLine { x: thumb.width - 0.5; y: thumb.height * 0.62 }
            PathLine { x: thumb.width / 2; y: thumb.height - 0.5 }
            PathLine { x: 0.5; y: thumb.height * 0.62 }
            PathLine { x: 0.5; y: 0.5 }
        }
    }

    function valueFromX(px) {
        const v = (px - root.thumbW / 2) / Math.max(1, root.travel) * 100;
        return Math.max(root.from, Math.min(root.to, Math.round(v)));
    }

    function drag(px) {
        root.dragValue = root.valueFromX(px);
        throttle.schedule(root.dragValue);
    }

    function finish() {
        throttle.release(root.dragValue >= 0 ? root.dragValue : undefined);
        if (!throttle.pending) {
            root.dragValue = -1;
        }
    }

    CommitThrottle {
        id: throttle

        reportedValue: root.value
        onCommitted: val => root.requested(val)
    }

    function nudge(delta) {
        if (!root.controllable) {
            return;
        }
        const base = throttle.pending ? throttle.pendingValue : root.shown;
        const next = Math.max(root.from, Math.min(root.to, base + delta));
        root.dragValue = next;
        throttle.nudge(next);
    }

    Keys.onLeftPressed: root.nudge(-5)
    Keys.onRightPressed: root.nudge(5)
    Keys.onDownPressed: root.nudge(-1)
    Keys.onUpPressed: root.nudge(1)
    Keys.onPressed: event => {
        if (!root.controllable) {
            return;
        }
        if (event.key === Qt.Key_PageDown) {
            root.nudge(-10);
            event.accepted = true;
        } else if (event.key === Qt.Key_PageUp) {
            root.nudge(10);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            root.nudge(root.from - root.shown);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.nudge(root.to - root.shown);
            event.accepted = true;
        }
    }

    Rectangle {
        visible: root.activeFocus
        anchors.fill: parent
        anchors.margins: -root.style.line
        radius: 2
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.focus
    }

    MouseArea {
        id: pointer

        anchors.fill: parent
        enabled: root.controllable
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.drag(mouse.x);
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.drag(mouse.x);
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
