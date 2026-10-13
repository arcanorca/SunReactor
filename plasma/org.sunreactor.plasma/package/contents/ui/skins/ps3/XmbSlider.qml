/*
 * A thin XMB slider: a hairline groove, a white fill with a soft under-glow, a
 * round thumb with a halo that grows when focused or pressed. The automatic
 * target is a small accent tick, the automatic range a faint band. `known:
 * false` draws the groove with no thumb or fill: an unknown value is never shown
 * as zero. Keys, wheel and accessibility are the stock slider's.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T

T.Slider {
    id: control

    required property XmbStyle style
    /*! Where the automatic curve wants the value; -1 for none. */
    property real target: -1
    /*! The automatic range; -1 for none. */
    property real limitMin: -1
    property real limitMax: -1
    /*! False when the daemon has no value for this slider. */
    property bool known: true

    readonly property real thumbDiameter: style.snap(style.unit * 0.62)
    readonly property real usable: availableWidth
    readonly property real centreY: height / 2
    readonly property real valueX: xAt(known ? value : from)

    function xAt(v) {
        return control.style.snap(control.leftPadding
            + (v - control.from) / Math.max(1, control.to - control.from) * control.usable);
    }

    from: 0
    to: 100
    stepSize: 1
    snapMode: T.Slider.SnapAlways
    wheelEnabled: false
    focusPolicy: Qt.StrongFocus

    leftPadding: thumbDiameter / 2
    rightPadding: thumbDiameter / 2
    topPadding: 0
    bottomPadding: 0

    implicitWidth: 160
    implicitHeight: style.sliderHeight

    background: Item {
        implicitWidth: control.implicitWidth
        implicitHeight: control.implicitHeight

        // The groove.
        Rectangle {
            x: control.leftPadding
            y: control.style.snap(control.centreY - height / 2)
            width: control.usable
            height: control.style.line2
            radius: height / 2
            color: control.style.track
        }

        // The automatic range.
        Rectangle {
            visible: control.limitMin >= 0 && control.limitMax > control.limitMin
            x: control.xAt(control.limitMin)
            y: control.style.snap(control.centreY - height / 2)
            width: Math.max(0, control.xAt(control.limitMax) - x)
            height: control.style.line2
            radius: height / 2
            color: control.style.alpha(control.style.ink, 0.28)
        }

        // The fill's soft under-glow, then the fill itself.
        Rectangle {
            visible: control.known
            x: control.leftPadding
            y: control.style.snap(control.centreY - height / 2)
            width: Math.max(0, control.valueX - x)
            height: control.style.snap(control.style.line2 * 3)
            radius: height / 2
            color: control.style.alpha(control.style.ink, control.enabled ? 0.18 : 0.06)
        }
        Rectangle {
            visible: control.known
            x: control.leftPadding
            y: control.style.snap(control.centreY - height / 2)
            width: Math.max(0, control.valueX - x)
            height: control.style.line2
            radius: height / 2
            color: control.enabled ? control.style.ink : control.style.inkDim
        }

        // The automatic target: a short accent tick across the groove.
        Rectangle {
            visible: control.target >= 0
            x: control.xAt(control.target) - width / 2
            y: control.style.snap(control.centreY - height / 2)
            width: control.style.line2
            height: control.style.snap(control.style.unit * 0.6)
            radius: width / 2
            color: control.style.accentSoft
        }
    }

    handle: Item {
        x: control.valueX - width / 2
        y: control.style.snap(control.centreY - height / 2)
        width: control.thumbDiameter
        height: width
        visible: control.known && control.enabled

        // The halo: a soft white disc behind the thumb.
        Rectangle {
            anchors.centerIn: parent
            width: parent.width * (control.visualFocus || control.pressed ? 2.5 : 1.8)
            height: width
            radius: width / 2
            color: control.style.alpha(control.style.ink,
                                       control.visualFocus || control.pressed ? 0.3 : 0.14)
        }

        Rectangle {
            anchors.fill: parent
            radius: width / 2
            color: control.style.ink
        }
    }
}
