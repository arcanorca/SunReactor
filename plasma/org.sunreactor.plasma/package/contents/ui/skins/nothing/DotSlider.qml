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
import org.sunreactor.plasma

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

    implicitWidth: style.unit * 8
    implicitHeight: bar.rows * bar.pitch
    activeFocusOnTab: live

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: accessibleDescription
    Accessible.focusable: live
    Accessible.onIncreaseAction: step(1)
    Accessible.onDecreaseAction: step(-1)

    readonly property int minimumValue: from
    readonly property int maximumValue: to

    function setLocal(v) {
        if (!live) {
            return;
        }
        editing = true;
        localValue = Math.max(from, Math.min(to, Math.round(v)));
        throttle.restart();
    }

    function step(delta) {
        if (!live) {
            return;
        }
        setLocal((editing ? localValue : value) + delta);
        settle.restart();
    }

    function finish() {
        throttle.stop();
        if (editing) {
            root.requested(localValue);
        }
        settle.restart();
    }

    function valueAtX(x) {
        return Math.round(Math.max(0, Math.min(root.width, x)) / Math.max(1, root.width) * 100);
    }

    // The daemon may answer late: hold the user's value for a while after the last change.
    Timer {
        id: throttle

        interval: 250
        onTriggered: root.requested(root.localValue)
    }

    Timer {
        id: settle

        interval: 1500
        onTriggered: {
            if (!pointer.pressed) {
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

    DotBar {
        id: bar

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        cols: Math.max(8, Math.floor(root.width / Math.max(1, bar.pitch)))
        rows: 2
        pitch: root.style.pitch + 1
        value: root.shown / 100
        thumbAccent: root.known
        litColor: root.enabled ? root.style.ink : root.style.muted
        accentColor: root.style.accent
        dimColor: root.style.dotGhost
    }

    // Keyboard focus: one hairline under the bar.
    Rectangle {
        visible: root.activeFocus
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: bar.bottom
        anchors.topMargin: root.style.gap * 0.5
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
            root.setLocal(root.valueAtX(mouse.x));
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.setLocal(root.valueAtX(mouse.x));
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
