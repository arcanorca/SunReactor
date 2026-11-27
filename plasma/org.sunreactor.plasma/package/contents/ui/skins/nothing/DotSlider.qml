/*
 * A slider drawn as a row of dots (DotBar): lit up to the value, the head is the
 * red thumb. Unknown values draw no thumb and take no input.
 *
 * Mouse: press or drag sets the value under the pointer; the value is reported
 * as `requested(value)` once the pointer has rested for 250 ms, and once more on
 * release (the instrument fader's rule, for DDC/CI). Keyboard: Left/Down -1,
 * Right/Up +1, Page Up/Down +-10, Home/End 0/100, each reported the same way.
 * Assistive tech sees a Slider with its value, range and step.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../components"

Item {
    id: root

    required property NothingStyle style
    /*! What the daemon shows, 0..100; -1 when unknown. */
    property int value: -1
    property int from: 0
    property int to: 100
    readonly property int stepSize: 1
    property string accessibleName: ""
    property string accessibleDescription: ""

    signal requested(int value)

    readonly property bool known: value >= 0
    readonly property bool live: known && enabled
    /*! What is drawn: the user's own value while they are at it, else the daemon's. */
    property bool editing: false
    property int localValue: 0
    readonly property int shown: editing ? localValue : Math.max(0, value)

    readonly property real trackH: style.snap(style.unit * 0.72)
    readonly property real ratio: Math.max(0, Math.min(1, (shown - from) / Math.max(1, to - from)))

    implicitWidth: style.unit * 8
    implicitHeight: trackH
    activeFocusOnTab: live

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: accessibleDescription
    Accessible.focusable: live
    Accessible.onIncreaseAction: step(1)
    Accessible.onDecreaseAction: step(-1)

    readonly property int minimumValue: from
    readonly property int maximumValue: to

    function setLocal(v, fromKey) {
        if (!live) {
            return;
        }
        const clamped = Math.max(from, Math.min(to, Math.round(v)));
        if (!editing) {
            editing = true;
            localValue = Math.max(0, value);
        }
        if (clamped === localValue) {
            return;
        }
        localValue = clamped;
        if (fromKey) {
            throttle.nudge(clamped);
        } else {
            throttle.schedule(clamped);
        }
    }

    function step(delta) {
        if (!live) {
            return;
        }
        setLocal((editing ? localValue : value) + delta, true);
        settle.restart();
    }

    function finish() {
        throttle.release(localValue);
        settle.restart();
    }

    function valueAtX(x) {
        return Math.round(Math.max(0, Math.min(root.width, x)) / Math.max(1, root.width) * 100);
    }

    // The daemon may answer late: hold the user's value for a while after the last change.
    CommitThrottle {
        id: throttle

        reportedValue: root.value
        onCommitted: val => root.requested(val)
    }

    Timer {
        id: settle

        interval: 1500
        onTriggered: {
            if (!pointer.pressed) {
                if (throttle.pending) {
                    throttle.release(root.localValue);
                }
                root.editing = false;
            }
        }
    }

    Keys.onPressed: event => {
        let delta = 0;
        switch (event.key) {
        case Qt.Key_Left:
        case Qt.Key_Down:
            delta = -1;
            break;
        case Qt.Key_Right:
        case Qt.Key_Up:
            delta = 1;
            break;
        case Qt.Key_PageDown:
            delta = -10;
            break;
        case Qt.Key_PageUp:
            delta = 10;
            break;
        case Qt.Key_Home:
            delta = root.from - root.shown;
            break;
        case Qt.Key_End:
            delta = root.to - root.shown;
            break;
        default:
            return;
        }
        event.accepted = true;
        root.step(delta);
    }

    // Nothing OS Quick-Settings tactile pill slider with dot-matrix scale ticks & red signal head
    Item {
        id: bar

        anchors.fill: parent

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            color: root.style.surfaceElevated
            border.width: root.style.hairline
            border.color: root.activeFocus ? root.style.ink : root.style.outline
            antialiasing: true
        }

        // Tactile dot ticks inside the track
        Row {
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: Math.round(root.trackH * 0.45)
            anchors.rightMargin: Math.round(root.trackH * 0.45)
            spacing: Math.max(4, Math.floor((width - 12 * 2) / 11))

            Repeater {
                model: 12

                delegate: Rectangle {
                    width: 2
                    height: 2
                    radius: 1
                    color: root.style.dotGhost
                }
            }
        }

        // Active bright white fill pill (at 0% collapses cleanly to the red signal dot)
        Rectangle {
            visible: root.known
            x: 1
            y: 1
            height: parent.height - 2
            width: root.ratio <= 0.01 ? height : Math.max(height, Math.round(root.ratio * (parent.width - 2)))
            radius: height / 2
            color: root.ratio <= 0.01 ? "transparent" : (root.enabled ? root.style.ink : root.style.muted)
            antialiasing: true

            // Signature Nothing red dot head inside the pill tip
            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                anchors.right: parent.right
                anchors.rightMargin: Math.max(3, Math.round(parent.height * 0.28))
                width: Math.max(4, Math.round(parent.height * 0.36))
                height: width
                radius: width / 2
                color: root.style.accent
            }
        }
    }

    // Keyboard focus: one hairline under the bar.
    Rectangle {
        visible: root.activeFocus
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bar.bottom
        anchors.topMargin: root.style.gap * 0.35
        height: root.style.hairline
        color: root.style.ink
    }

    MouseArea {
        id: pointer

        anchors.fill: parent
        anchors.topMargin: -root.style.gap * 0.5
        anchors.bottomMargin: -root.style.gap * 0.5
        enabled: root.live
        cursorShape: root.live ? Qt.PointingHandCursor : Qt.ArrowCursor
        preventStealing: true
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.setLocal(root.valueAtX(mouse.x), false);
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.setLocal(root.valueAtX(mouse.x), false);
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
