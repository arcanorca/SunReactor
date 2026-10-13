/*
 * A Workbench proportional gadget: a recessed trough with an orange knob. The
 * knob sits on whole art pixels (nine wide, from 1 px to the far end), so no
 * fractional position is ever drawn. Mouse drag, or Left/Right (1 %),
 * Up/Down (5 %), PageUp/PageDown (10 %), Home/End. A change is committed at
 * most every 250 ms while the user is still moving, and once when they let go.
 * value -1 means unknown: the knob rests at the start and nothing is shown.
 * minMark / maxMark (-1 = none) are the range limits, drawn as ticks.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "AmigaChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property AmigaStyle style
    property int value: -1
    property int minMark: -1
    property int maxMark: -1
    property string accessibleName: ""
    property bool editing: false
    property int localValue: 0

    readonly property int shown: editing ? localValue : Math.max(0, value)
    readonly property bool known: value >= 0
    /*! Knob travel in art px: the trough inside its 1 px frames, less the knob. */
    readonly property int travel: Math.max(0, width - 2 - 9)

    signal committed(int value)

    implicitHeight: 8
    height: 8

    function xOf(percent) {
        return 1 + Math.round(Math.max(0, Math.min(100, percent)) / 100 * travel);
    }
    function valueAt(artX) {
        if (travel <= 0) {
            return 0;
        }
        return Math.round(Math.max(0, Math.min(travel, artX - 1 - 4)) / travel * 100);
    }
    function setLocal(v) {
        const clamped = Math.max(0, Math.min(100, v));
        if (!editing) {
            editing = true;
        }
        localValue = clamped;
        throttle.restart();
    }
    function step(delta) {
        if (!enabled) {
            return;
        }
        setLocal((editing ? localValue : Math.max(0, value)) + delta);
        finish();
    }
    function finish() {
        throttle.stop();
        if (editing) {
            root.committed(localValue);
            editing = false;
        }
    }

    Timer {
        id: throttle
        interval: 250
        onTriggered: root.committed(root.localValue)
    }

    Accessible.role: Accessible.Slider
    Accessible.name: accessibleName
    Accessible.description: known ? shown + " %" : ""
    activeFocusOnTab: true

    Keys.onLeftPressed: root.step(-1)
    Keys.onRightPressed: root.step(1)
    Keys.onDownPressed: root.step(-5)
    Keys.onUpPressed: root.step(5)
    Keys.onPressed: event => {
        if (event.key === Qt.Key_PageUp) {
            root.step(10);
            event.accepted = true;
        } else if (event.key === Qt.Key_PageDown) {
            root.step(-10);
            event.accepted = true;
        } else if (event.key === Qt.Key_Home) {
            root.setLocal(0);
            root.finish();
            event.accepted = true;
        } else if (event.key === Qt.Key_End) {
            root.setLocal(100);
            root.finish();
            event.accepted = true;
        }
    }

    Repeater {
        model: Chrome.troughRects(root.width, { face: root.style.face, ink: root.style.ink, body: root.style.body })
        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }

    Repeater {
        model: root.known ? Chrome.knobRects(root.xOf(root.shown), { ink: root.style.ink, hot: root.style.hot,
                                                                     body: root.style.body }) : []
        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }

    Repeater {
        model: [root.minMark, root.maxMark].filter(v => v >= 0)
        delegate: Rectangle {
            required property int modelData
            x: root.xOf(modelData) + 4
            y: 7
            width: 1
            height: 1
            color: root.style.ink
            antialiasing: false
        }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
        onPressed: mouse => {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.setLocal(root.valueAt(mouse.x));
        }
        onPositionChanged: mouse => {
            if (pressed) {
                root.setLocal(root.valueAt(mouse.x));
            }
        }
        onReleased: root.finish()
        onCanceled: root.finish()
    }
}
