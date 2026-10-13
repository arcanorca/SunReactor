/*
 * A channel slider, Wii menu style: a round-ended light grey track, a Wii blue
 * fill with a gloss over its top half, and a big white round thumb with a grey
 * ring and a faint shadow. A small dot marks where the automatic curve wants
 * the display to be.
 *
 * It reports what the user asked for and knows nothing of the daemon: while
 * dragged the thumb follows the pointer and `requested` is sent throttled (the
 * owner's writes are rate limited) and once more on release. Keys move it one
 * step (ten with PageUp and PageDown). It has the Slider accessible role, so
 * modules/ModeControl.qml can focus it.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property WiiStyle style
    property real from: 0
    property real to: 100
    /*! The value the owner shows (the override, else the applied value). */
    property real value: 0
    /*! False until the daemon has reported a value: no thumb, dimmed track. */
    property bool known: true
    /*! Where the automatic curve is now; -1 for none. */
    property real target: -1

    /*! The value the user asked for: throttled while dragging, and on release or key. */
    signal requested(int value)

    readonly property bool dragging: pointer.pressed
    property real dragValue: 0
    readonly property real shown: dragging ? dragValue : value

    readonly property real thumbD: style.snap(style.unit * 1.2)
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
        const v = Math.max(from, Math.min(to, Math.round(root.value) + step));
        root.requested(v);
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
            root.requested(root.from);
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.requested(root.to);
            event.accepted = true;
        }
    }

    Timer {
        id: throttle

        interval: 250
        onTriggered: root.requested(Math.round(root.dragValue))
    }

    // The track: light grey, round-ended.
    Rectangle {
        id: track

        x: root.trackX
        y: (root.height - root.trackH) / 2
        width: root.trackW
        height: root.trackH
        radius: height / 2
        antialiasing: true
        color: root.style.track
    }

    // The fill: Wii blue, with a gloss over its top half.
    Item {
        id: fillBox

        x: track.x
        y: track.y
        width: root.known ? Math.max(track.height, root.ratio * track.width) : 0
        height: track.height
        visible: root.known

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            antialiasing: true
            color: root.style.accent
        }
        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            height: Math.round(parent.height / 2)
            radius: height / 2
            antialiasing: true
            gradient: Gradient {
                GradientStop { position: 0; color: root.style.gloss }
                GradientStop { position: 1; color: root.style.glossClear }
            }
        }
    }

    // Where the automatic curve wants the display: a small dot under the track.
    Rectangle {
        visible: root.known && root.target >= 0
        width: root.style.snap(root.style.unit * 0.36)
        height: width
        radius: width / 2
        x: root.xOf(root.target) - width / 2
        y: track.y + track.height + root.style.gap / 2 - height / 2
        antialiasing: true
        color: root.style.inkSoft
    }

    // The thumb: a big white round knob with a grey ring and a faint shadow.
    Item {
        id: thumb

        visible: root.known
        width: root.thumbD
        height: root.thumbD
        x: root.xOf(root.shown) - width / 2
        y: (root.height - height) / 2

        Rectangle {
            x: 0
            y: root.style.snap(1.5)
            width: parent.width
            height: parent.height
            radius: width / 2
            antialiasing: true
            color: root.style.alpha(root.style.ink, 0.14)
        }
        Rectangle {
            anchors.fill: parent
            radius: width / 2
            antialiasing: true
            color: root.style.paper
            border.width: root.style.line2
            border.color: root.dragging ? root.style.accent : root.style.ring
        }
    }

    // Keyboard focus: a blue ring around the thumb.
    Rectangle {
        visible: root.known && root.activeFocus
        x: thumb.x - root.style.line2 * 2
        y: thumb.y - root.style.line2 * 2
        width: thumb.width + root.style.line2 * 4
        height: thumb.height + root.style.line2 * 4
        radius: width / 2
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.accent
    }

    MouseArea {
        id: pointer

        anchors.fill: parent
        enabled: root.known
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.dragValue = root.valueAt(mouse.x);
        }
        onPositionChanged: mouse => {
            if (pointer.pressed) {
                root.dragValue = root.valueAt(mouse.x);
                throttle.restart();
            }
        }
        onReleased: {
            throttle.stop();
            root.requested(Math.round(root.dragValue));
        }
    }
}
