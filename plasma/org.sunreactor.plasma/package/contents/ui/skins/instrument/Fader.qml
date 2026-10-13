/*
 * A brightness fader drawn as instrument hardware, with the behaviour of a
 * stock slider (keys, wheel off, accessibility). It shows three things:
 *
 *   value        the thumb / lit length (what is set)
 *   target       a marker where the automatic curve wants to be (-1: none)
 *   limitMin/Max the automatic range on the scale (-1: none)
 *
 * Styles (InstrumentStyle.faderStyle): cap (Braun console fader with an
 * accent index line), trackpoint (ThinkPad red nub), segments (VFD / CRT
 * bar meter with ghost segments), tube (neon bargraph tube), trace (scope
 * cursor on a phosphor line). `known: false` draws the empty instrument and
 * hides the thumb: an unknown value is never shown as 0.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import QtQuick.Templates as T

T.Slider {
    id: control

    required property InstrumentStyle style
    property real target: -1
    property real limitMin: -1
    property real limitMax: -1
    property bool known: true
    property bool showScale: true
    property string faderStyle: style.faderStyle
    property color litColor: style.ink

    readonly property real zone: style.snap(style.sizeLegend * 1.7)
    readonly property real usable: availableWidth
    readonly property bool hasRange: limitMin >= 0 && limitMax >= limitMin

    function xAt(v) {
        return style.snap(leftPadding + (v - from) / Math.max(1, to - from) * usable);
    }

    from: 0
    to: 100
    stepSize: 1
    snapMode: T.Slider.SnapAlways
    wheelEnabled: false
    focusPolicy: Qt.StrongFocus

    leftPadding: style.snap(faderStyle === "cap" ? 5 : (faderStyle === "trackpoint" ? 6 : 0))
    rightPadding: leftPadding
    topPadding: 0
    bottomPadding: 0

    implicitWidth: 200
    implicitHeight: style.snap((showScale ? scale.implicitHeight + style.hairline * 2 : 0) + zone)

    readonly property real trackY: (showScale ? scale.implicitHeight + style.hairline * 2 : 0)
    readonly property real centreY: style.snap(trackY + zone / 2)
    readonly property real valueX: xAt(known ? value : from)

    background: Item {
        implicitWidth: control.implicitWidth
        implicitHeight: control.implicitHeight

        TickScale {
            id: scale

            visible: control.showScale
            x: control.leftPadding
            width: control.usable + control.style.hairline
            style: control.style
            minor: 10
            major: 50
            ticksDown: false
        }

        Loader {
            anchors.fill: parent
            sourceComponent: {
                switch (control.faderStyle) {
                case "segments":
                    return segmentsLook;
                case "tube":
                    return tubeLook;
                case "trace":
                    return traceLook;
                default:
                    return grooveLook;
                }
            }
        }
    }

    handle: Item {
        x: control.valueX - width / 2
        y: control.trackY
        width: control.faderStyle === "cap" ? control.style.snap(8) : control.style.snap(12)
        height: control.zone
        visible: control.known && control.enabled

        // Braun: a small console cap with an accent index line.
        Rectangle {
            visible: control.faderStyle === "cap"
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width
            height: parent.height
            radius: Math.min(2, control.style.radius)
            antialiasing: radius > 0
            color: control.pressed ? control.style.ink : control.style.capFace
            border.width: control.style.hairline
            border.color: control.visualFocus ? control.style.theme.borderActive : control.style.capShade

            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                y: control.style.snap(parent.height * 0.2)
                width: control.style.line2
                height: parent.height - y * 2
                color: control.style.accent
            }
        }

        // ThinkPad: the red TrackPoint nub.
        Rectangle {
            visible: control.faderStyle === "trackpoint"
            anchors.centerIn: parent
            width: control.style.snap(12)
            height: width
            radius: width / 2
            antialiasing: true
            color: control.style.accent
            border.width: control.style.line
            border.color: control.visualFocus ? control.style.ink
                : control.style.theme.mix(control.style.accent, control.style.panel, 0.45)

            Rectangle {
                anchors.centerIn: parent
                width: parent.width * 0.42
                height: width
                radius: width / 2
                antialiasing: true
                color: control.style.theme.mix(control.style.accent, control.style.panel, control.pressed ? 0.0 : 0.22)
            }
        }

        // Emissive styles: only a focus caret (the lit length is the value).
        Rectangle {
            visible: control.visualFocus && ["segments", "tube", "trace"].indexOf(control.faderStyle) >= 0
            anchors.horizontalCenter: parent.horizontalCenter
            y: parent.height + control.style.hairline
            width: control.style.snap(6)
            height: control.style.line2
            color: control.style.theme.borderActive
        }
    }

    // --- looks ---------------------------------------------------------------------

    // Groove for cap and trackpoint: hairline groove, range in a stronger
    // tone, fill up to the value, target as a hanging tick.
    Component {
        id: grooveLook

        Item {
            readonly property real thick: control.faderStyle === "cap" ? control.style.line2 : control.style.line
            Rectangle {
                x: control.xAt(control.from)
                y: control.style.snap(control.centreY - parent.thick / 2)
                width: control.xAt(control.to) - x + control.style.hairline
                height: parent.thick
                color: control.style.rule
            }
            Rectangle {
                visible: control.hasRange
                x: control.xAt(control.limitMin)
                y: control.style.snap(control.centreY + control.zone / 2 - control.style.line2)
                width: control.xAt(control.limitMax) - x + control.style.hairline
                height: control.style.hairline
                color: control.style.inkFaint
            }
            Repeater {
                model: control.hasRange ? [control.limitMin, control.limitMax] : []
                delegate: Rectangle {
                    required property real modelData
                    x: control.xAt(modelData)
                    y: control.style.snap(control.centreY + control.zone / 2 - control.style.line2 - height)
                    width: control.style.hairline
                    height: control.style.snap(3)
                    color: control.style.inkFaint
                }
            }
            Rectangle {
                visible: control.known
                x: control.xAt(control.from)
                y: control.style.snap(control.centreY - height / 2)
                width: Math.max(0, control.valueX - x)
                height: control.faderStyle === "cap" ? control.style.line2 : control.style.line2
                color: control.enabled ? (control.faderStyle === "cap" ? control.style.inkMuted : control.style.ink)
                                       : control.style.rule
            }
            // Automatic target: a small notch above the groove.
            Rectangle {
                visible: control.target >= 0
                x: control.xAt(control.target)
                y: control.trackY
                width: control.style.line
                height: control.style.snap(control.zone / 2 - parent.thick)
                color: control.style.accent2
            }
        }
    }

    // VFD / CRT bar meter: one segment per 2 %, unlit segments ghosted.
    Component {
        id: segmentsLook

        Item {
            id: meter

            readonly property int count: Math.max(10, Math.min(50, Math.floor(control.usable / control.style.snap(5))))
            readonly property real pitch: control.usable / count
            readonly property real segW: Math.max(control.style.hairline, control.style.floorSnap(pitch * 0.68))
            readonly property real segH: control.style.snap(control.zone * 0.62)
            readonly property real segY: control.style.snap(control.trackY + (control.zone - segH) / 2 - control.style.hairline * 2)
            readonly property int litCount: control.known ? Math.round((control.value - control.from) / (control.to - control.from) * count) : 0

            function rects(first, last, y, h) {
                let p = "";
                for (let i = first; i < last; ++i) {
                    const x = control.style.snap(control.leftPadding + i * pitch);
                    p += "M" + x + " " + y + " h" + segW + " v" + h + " h" + (-segW) + " Z ";
                }
                return p;
            }
            function indexOf(v) {
                return Math.max(0, Math.min(count - 1, Math.round((v - control.from) / (control.to - control.from) * count) - 1));
            }

            Shape {
                anchors.fill: parent
                antialiasing: false
                ShapePath {
                    strokeWidth: -1
                    fillColor: control.style.ghost
                    PathSvg { path: meter.rects(meter.litCount, meter.count, meter.segY, meter.segH) }
                }
                // Automatic range: a dim rail under the segments.
                ShapePath {
                    strokeWidth: -1
                    fillColor: control.hasRange ? control.style.theme.alpha(control.style.ink, 0.28) : "transparent"
                    PathSvg {
                        path: control.hasRange
                            ? meter.rects(meter.indexOf(control.limitMin), meter.indexOf(control.limitMax) + 1,
                                          control.style.snap(meter.segY + meter.segH + control.style.hairline * 2),
                                          control.style.hairline)
                            : ""
                    }
                }
            }

            Glow {
                anchors.fill: litShape
                source: litShape
                strength: control.style.glow
            }

            Shape {
                id: litShape

                anchors.fill: parent
                antialiasing: false
                ShapePath {
                    strokeWidth: -1
                    fillColor: control.enabled ? control.litColor : control.style.inkMuted
                    PathSvg { path: meter.rects(0, meter.litCount, meter.segY, meter.segH) }
                }
                // Automatic target: the segment cap above the bar.
                ShapePath {
                    strokeWidth: -1
                    fillColor: control.target >= 0 ? control.style.accent2 : "transparent"
                    PathSvg {
                        path: control.target >= 0
                            ? meter.rects(meter.indexOf(control.target), meter.indexOf(control.target) + 1,
                                          control.style.snap(meter.segY - control.style.line2 - control.style.hairline * 2),
                                          control.style.line2)
                            : ""
                    }
                }
            }
        }
    }

    // Neon bargraph tube (IN-9): a glass tube with a glowing column.
    Component {
        id: tubeLook

        Item {
            id: tube

            readonly property real tubeH: control.style.snap(control.zone * 0.7)
            readonly property real tubeY: control.style.snap(control.centreY - tubeH / 2)

            Rectangle {
                x: control.xAt(control.from)
                y: tube.tubeY
                width: control.xAt(control.to) - x
                height: tube.tubeH
                radius: height / 2
                antialiasing: true
                border.width: control.style.hairline
                border.color: control.style.theme.alpha(control.style.ink, 0.16)
                gradient: Gradient {
                    GradientStop { position: 0.0; color: control.style.theme.alpha(control.style.ink, 0.08) }
                    GradientStop { position: 0.5; color: control.style.theme.alpha(control.style.ink, 0.02) }
                    GradientStop { position: 1.0; color: control.style.theme.alpha(control.style.ink, 0.05) }
                }
            }
            // Cold cathode wire along the tube.
            Rectangle {
                x: control.xAt(control.from) + tube.tubeH / 2
                y: control.style.snap(control.centreY)
                width: control.xAt(control.to) - x - tube.tubeH / 2
                height: control.style.hairline
                color: control.style.ghost
            }
            Repeater {
                model: control.hasRange ? [control.limitMin, control.limitMax] : []
                delegate: Rectangle {
                    required property real modelData
                    x: control.xAt(modelData)
                    y: tube.tubeY + tube.tubeH + control.style.hairline * 2
                    width: control.style.hairline
                    height: control.style.snap(3)
                    color: control.style.inkFaint
                }
            }
            Rectangle {
                visible: control.target >= 0
                x: control.xAt(control.target)
                y: tube.tubeY - height - control.style.hairline * 2
                width: control.style.line
                height: control.style.snap(3)
                color: control.style.accent2
            }

            Glow {
                anchors.fill: column
                source: column
                strength: control.style.glow
            }

            Rectangle {
                id: column

                visible: control.known
                x: control.xAt(control.from) + control.style.snap(tube.tubeH * 0.3)
                y: control.style.snap(control.centreY - height / 2)
                width: Math.max(0, control.valueX - x)
                height: control.style.snap(tube.tubeH * 0.38)
                radius: height / 2
                antialiasing: true
                color: control.enabled ? control.litColor : control.style.inkMuted
            }
        }
    }

    // Oscilloscope: a phosphor line from 0 to the value, cursor at the value.
    Component {
        id: traceLook

        Item {
            Repeater {
                // Dotted graticule line: one dot every 2 % of the scale.
                model: 51
                delegate: Rectangle {
                    required property int index
                    x: control.xAt(index * 2)
                    y: control.centreY
                    width: control.style.hairline
                    height: control.style.hairline
                    color: control.style.ruleStrong
                }
            }
            Rectangle {
                visible: control.target >= 0
                x: control.xAt(control.target)
                y: control.trackY
                width: control.style.hairline
                height: control.zone
                color: control.style.inkFaint
            }

            Glow {
                anchors.fill: trace
                source: trace
                strength: control.style.glow
            }

            Item {
                id: trace

                anchors.fill: parent
                visible: control.known

                Rectangle {
                    x: control.xAt(control.from)
                    y: control.style.snap(control.centreY - height / 2)
                    width: Math.max(0, control.valueX - x)
                    height: control.style.line2
                    color: control.enabled ? control.litColor : control.style.inkMuted
                }
                Rectangle {
                    x: control.valueX
                    y: control.trackY
                    width: control.style.line
                    height: control.zone
                    color: control.enabled ? control.style.accent : control.style.inkMuted
                }
            }
        }
    }
}
