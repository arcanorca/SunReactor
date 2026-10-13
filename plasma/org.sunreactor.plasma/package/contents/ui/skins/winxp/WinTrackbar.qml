/*
 * An XP trackbar: a sunken groove filled with the green blocks of XP's progress
 * bar up to the value, and the pentagon pointer at the value. A press or a drag
 * sets the value; `requested` comes at most every 160 ms while dragging and once
 * on release. Left and Right move it by 5.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    required property WinStyle style
    /*! 0..100. */
    property int value: 0
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
    readonly property int shown: dragValue >= 0 ? dragValue : Math.max(0, Math.min(100, value))
    readonly property real fillLength: shown / 100 * travel

    signal requested(int value)

    implicitWidth: thumbW * 6
    implicitHeight: thumbH
    opacity: controllable ? 1 : 0.45
    activeFocusOnTab: true

    Accessible.role: Accessible.Slider
    Accessible.name: i18nd("plasma_applet_org.sunreactor.plasma", "Brightness")

    onValueChanged: dragValue = -1

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
        return Math.round(Math.max(0, Math.min(100, v)));
    }

    property bool pending: false

    function emitDrag() {
        root.requested(root.dragValue);
        root.pending = false;
        throttle.restart();
    }

    function drag(px) {
        root.dragValue = root.valueFromX(px);
        if (throttle.running) {
            root.pending = true;
        } else {
            root.emitDrag();
        }
    }

    function finish() {
        throttle.stop();
        root.pending = false;
        if (root.dragValue >= 0) {
            root.requested(root.dragValue);
        }
    }

    Timer {
        id: throttle

        interval: 160
        repeat: false
        onTriggered: {
            if (root.pending) {
                root.emitDrag();
            }
        }
    }

    Keys.onLeftPressed: {
        if (root.controllable) {
            root.requested(Math.max(0, root.shown - 5));
        }
    }
    Keys.onRightPressed: {
        if (root.controllable) {
            root.requested(Math.min(100, root.shown + 5));
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.controllable
        cursorShape: Qt.PointingHandCursor
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
