/*
 * A brightness slider as a neon tube: an unlit tube, the lit part up to the
 * value, the automatic limits as a cyan band on the tube, the curve's target as
 * a tick, and a round thumb with a glow. It is a stock slider underneath
 * (keys, accessibility, drag), so the module code treats it like any other.
 *
 * `known: false` draws the empty tube and hides the thumb: an unknown value is
 * never shown as 0.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T
import "../instrument" as Instrument

T.Slider {
    id: control

    required property SynthStyle style
    property color neon: style.pink
    /*! Where the automatic curve wants the value now (-1: none). */
    property real target: -1
    /*! The automatic range (-1: none). */
    property real limitMin: -1
    property real limitMax: -1
    property bool known: true

    readonly property real thumbSize: style.snap(style.unit * 0.66)
    readonly property real tubeSize: style.snap(Math.max(3, style.unit * 0.2))
    readonly property real travel: Math.max(0, width - thumbSize)
    readonly property real midY: height / 2

    /*! Pixel x of a value: the same mapping as the stock handle position. */
    function xAt(v) {
        const clamped = Math.max(control.from, Math.min(control.to, v));
        return thumbSize / 2 + travel * (clamped - control.from) / Math.max(1, control.to - control.from);
    }

    from: 0
    to: 100
    stepSize: 1
    implicitWidth: style.unit * 10
    implicitHeight: style.snap(style.unit * 1.0)

    background: Item {
        // The unlit tube, the whole travel.
        Rectangle {
            x: control.thumbSize / 2
            width: control.travel
            height: control.tubeSize
            y: control.midY - height / 2
            radius: height / 2
            antialiasing: true
            color: control.style.alpha(control.style.ink, 0.13)
        }

        // The automatic range: a cyan band on the tube.
        Rectangle {
            visible: control.limitMin >= 0 && control.limitMax >= control.limitMin
            x: control.xAt(control.limitMin)
            width: Math.max(0, control.xAt(control.limitMax) - x)
            height: control.tubeSize
            y: control.midY - height / 2
            color: control.style.alpha(control.style.cyan, 0.5)
        }

        // The lit tube, up to the value.
        Rectangle {
            visible: control.known
            x: control.thumbSize / 2
            width: Math.max(0, control.xAt(control.value) - x)
            height: control.tubeSize
            y: control.midY - height / 2
            radius: height / 2
            antialiasing: true
            color: control.neon
        }

        // The curve's target: a short cyan tick across the tube.
        Rectangle {
            visible: control.target >= 0
            x: Math.round(control.xAt(control.target)) - control.style.line2 / 2
            width: control.style.line2
            height: control.style.snap(control.thumbSize * 0.7)
            y: control.midY - height / 2
            color: control.style.cyan
        }
    }

    handle: Item {
        id: knob

        // Placed by the same mapping as the tube, so the thumb sits on the lit end.
        x: control.xAt(control.value) - width / 2
        y: control.midY - height / 2
        width: control.thumbSize
        height: control.thumbSize
        visible: control.known

        // The glow sits underneath the thumb; it is static (drawn when the value changes).
        Instrument.Glow {
            anchors.fill: face
            source: face
            strength: control.style.glow
            tint: control.neon
        }

        Rectangle {
            id: face

            anchors.fill: parent
            radius: width / 2
            antialiasing: true
            color: control.style.night
            border.width: control.style.line2
            border.color: control.neon

            // A bright core, so the thumb reads as a lit bulb, not a ring.
            Rectangle {
                anchors.centerIn: parent
                width: parent.width * 0.34
                height: width
                radius: width / 2
                color: control.neon
            }
        }
    }
}
