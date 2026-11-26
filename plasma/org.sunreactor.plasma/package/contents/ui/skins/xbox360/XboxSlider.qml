/*
 * Xbox 360 metallic grooved slider with glowing green fill and chrome-green
 * orb thumb (assets/xbox_slider_thumb.png).
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../components"

Item {
    id: root

    required property XboxStyle style
    property real from: 0
    property real to: 100
    property real value: 0
    property bool known: true
    property real target: -1

    signal requested(int value)

    readonly property bool dragging: pointer.pressed
    property real dragValue: 0
    readonly property real shown: dragging ? dragValue
        : (throttle.pending ? throttle.pendingValue : value)

    readonly property real thumbD: style.snap(style.unit * 1.15)
    readonly property real trackH: style.snap(style.unit * 0.42)
    readonly property real trackX: thumbD / 2
    readonly property real trackW: Math.max(1, width - thumbD)
    readonly property real ratio: Math.max(0, Math.min(1, (shown - from) / Math.max(1, to - from)))

    function xOf(v) {
        return trackX + Math.max(0, Math.min(1, (v - from) / Math.max(1, to - from))) * trackW;
    }
    function valueAt(x) {
        const r = Math.max(0, Math.min(1, (x - trackX) / trackW));
        return Math.round(from + r * (to - from));
    }
    function nudge(step) {
        if (!root.known) {
            return;
        }
        const base = throttle.pending ? throttle.pendingValue : Math.round(root.value);
        const v = Math.max(from, Math.min(to, base + step));
        throttle.nudge(v);
    }

    implicitWidth: 120
    implicitHeight: thumbD + style.gap
    opacity: known ? 1 : 0.5
    activeFocusOnTab: true

    Accessible.role: Accessible.Slider
    Accessible.onIncreaseAction: root.nudge(1)
    Accessible.onDecreaseAction: root.nudge(-1)

    Keys.onLeftPressed: root.nudge(-1)
    Keys.onDownPressed: root.nudge(-1)
    Keys.onRightPressed: root.nudge(1)
    Keys.onUpPressed: root.nudge(1)
    Keys.onPressed: event => {
        if (event.key === Qt.Key_PageDown) {
            root.nudge(-10);
            event.accepted = true;
        } else if (event.key === Qt.Key_PageUp) {
            root.nudge(10);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            throttle.nudge(root.from);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            throttle.nudge(root.to);
            event.accepted = true;
        }
    }

    CommitThrottle {
        id: throttle

        reportedValue: Math.round(root.value)
        onCommitted: val => root.requested(val)
    }

    // Recessed metallic groove
    Rectangle {
        id: track

        x: root.trackX
        y: (root.height - root.trackH) / 2
        width: root.trackW
        height: root.trackH
        radius: height / 2
        color: root.style.grooveFill
        border.width: 1
        border.color: root.style.grooveEdge
    }

    // Glowing Xbox green fill
    Rectangle {
        x: track.x + 1
        y: track.y + 1
        width: root.known ? Math.max(0, root.ratio * (track.width - 2)) : 0
        height: Math.max(2, track.height - 2)
        radius: height / 2
        visible: root.known && width > 0
        gradient: Gradient {
            GradientStop { position: 0; color: root.style.grooveGlowTop }
            GradientStop { position: 1; color: root.style.grooveGlowBottom }
        }
    }

    // Target marker dot under track
    Rectangle {
        visible: root.known && root.target >= 0
        width: root.style.snap(root.style.unit * 0.32)
        height: width
        radius: width / 2
        x: root.xOf(root.target) - width / 2
        y: track.y + track.height + root.style.gap * 0.45 - height / 2
        color: root.style.greenBright
    }

    // Chrome-and-green orb thumb
    Image {
        id: thumb

        visible: root.known
        width: root.thumbD
        height: root.thumbD
        x: root.xOf(root.shown) - width / 2
        y: (root.height - height) / 2
        source: Qt.resolvedUrl(root.style.asset("xbox_slider_thumb.png"))
        smooth: true
        mipmap: true
    }

    Rectangle {
        visible: root.known && root.activeFocus
        x: thumb.x - root.style.line2
        y: thumb.y - root.style.line2
        width: thumb.width + 2 * root.style.line2
        height: thumb.height + 2 * root.style.line2
        radius: width / 2
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.greenBright
    }

    MouseArea {
        id: pointer

        anchors.fill: parent
        enabled: root.known
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.dragValue = root.valueAt(mouse.x);
            throttle.schedule(Math.round(root.dragValue));
        }
        onPositionChanged: mouse => {
            if (pointer.pressed) {
                root.dragValue = root.valueAt(mouse.x);
                throttle.schedule(Math.round(root.dragValue));
            }
        }
        onReleased: throttle.release(Math.round(root.dragValue))
        onCanceled: throttle.release(Math.round(root.dragValue))
    }
}
