/*
 * A brightness slider as Control Center draws it: a tall pill with a near-white
 * fill up to the value, a sun inside its left end, and no thumb (the end of the
 * fill is the thumb). Drags write one request per 250 ms while moving and one on
 * release, like the instrument faders; the daemon's value is shown again after
 * release. Keys, the wheel is off, and the slider role come from the Quick
 * Controls Slider. Unknown values draw an empty track, never a zero fill.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T

T.Slider {
    id: control

    required property MacStyle style
    /*! The value shown, in percent; -1 while the daemon has not reported one. */
    property int percent: -1
    property int trackHeight: style.sliderHeight
    /*! The sun glyph sits inside the pill (brightness sliders; range sliders not). */
    property bool showSun: true

    readonly property bool hasValue: percent >= 0

    signal requested(int percent)

    from: 0
    to: 100
    stepSize: 1
    snapMode: T.Slider.SnapAlways
    wheelEnabled: false
    focusPolicy: Qt.StrongFocus
    value: hasValue ? percent : 0

    leftPadding: 0
    rightPadding: 0
    topPadding: 0
    bottomPadding: 0

    implicitWidth: 200
    implicitHeight: trackHeight

    Timer {
        id: throttle

        interval: 250
        onTriggered: control.requested(Math.round(control.value))
    }

    onMoved: throttle.restart()
    onPressedChanged: {
        if (pressed) {
            return;
        }
        throttle.stop();
        control.requested(Math.round(control.value));
        // The user's drag is over: show the daemon's value again.
        control.value = Qt.binding(() => control.hasValue ? control.percent : 0);
    }

    background: Item {
        implicitWidth: 200
        implicitHeight: control.trackHeight

        Rectangle {
            anchors.fill: parent
            radius: height / 2
            antialiasing: true
            color: control.style.trackRest
        }

        Rectangle {
            id: fill

            visible: control.hasValue
            x: 0
            y: 0
            width: control.visualPosition * parent.width
            height: parent.height
            radius: height / 2
            antialiasing: true
            color: control.style.trackFill
        }

        SunGlyph {
            id: sun

            visible: control.showSun
            x: control.style.snap(control.trackHeight * 0.32)
            anchors.verticalCenter: parent.verticalCenter
            size: control.style.snap(control.trackHeight * 0.5)
            color: control.hasValue && fill.width > x + size ? control.style.panel : control.style.sunOff
        }

        // Keyboard focus: an accent ring round the pill.
        Rectangle {
            anchors.fill: parent
            visible: control.visualFocus
            radius: height / 2
            color: "transparent"
            border.width: control.style.hairline * 2
            border.color: control.style.accent
            antialiasing: true
        }
    }
}
