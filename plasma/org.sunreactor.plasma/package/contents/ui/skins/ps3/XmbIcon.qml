/*
 * The XMB category glyphs, drawn with Shapes on a 24-unit grid and scaled to
 * `size`: "sun" (brightness: a disc with eight rays) and "cloud" (weather).
 * Solid colour, no outline; the caller picks the tone.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    /*! "sun" or "cloud". */
    property string kind: "sun"
    property color color: "white"
    /*! Logical px of the square glyph. */
    property real size: 20

    readonly property real u: size / 24

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    /*! The eight sun rays, as line segments on the grid. */
    readonly property string rays: {
        let d = "";
        for (let k = 0; k < 8; ++k) {
            const a = k * Math.PI / 4;
            const x0 = (12 + Math.cos(a) * 7.6) * u;
            const y0 = (12 + Math.sin(a) * 7.6) * u;
            const x1 = (12 + Math.cos(a) * 10.4) * u;
            const y1 = (12 + Math.sin(a) * 10.4) * u;
            d += "M " + x0.toFixed(2) + " " + y0.toFixed(2) + " L " + x1.toFixed(2) + " " + y1.toFixed(2) + " ";
        }
        return d;
    }

    Shape {
        anchors.fill: parent
        visible: root.kind === "sun"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.color
            PathAngleArc {
                centerX: 12 * root.u
                centerY: 12 * root.u
                radiusX: 4.6 * root.u
                radiusY: 4.6 * root.u
                startAngle: 0
                sweepAngle: 360
            }
        }

        ShapePath {
            fillColor: "transparent"
            strokeColor: root.color
            strokeWidth: Math.max(1.5, 1.9 * root.u)
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.rays }
        }
    }

    Shape {
        anchors.fill: parent
        visible: root.kind === "cloud"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.color
            PathAngleArc {
                centerX: 8.0 * root.u
                centerY: 14.4 * root.u
                radiusX: 3.8 * root.u
                radiusY: 3.8 * root.u
                startAngle: 0
                sweepAngle: 360
            }
        }

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.color
            PathAngleArc {
                centerX: 13.4 * root.u
                centerY: 11.2 * root.u
                radiusX: 5.2 * root.u
                radiusY: 5.2 * root.u
                startAngle: 0
                sweepAngle: 360
            }
        }

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.color
            PathSvg {
                path: "M " + (6.6 * root.u).toFixed(2) + " " + (14.4 * root.u).toFixed(2)
                    + " H " + (16.0 * root.u).toFixed(2)
                    + " A " + (2.4 * root.u).toFixed(2) + " " + (2.4 * root.u).toFixed(2) + " 0 0 1 "
                    + (16.0 * root.u).toFixed(2) + " " + (19.2 * root.u).toFixed(2)
                    + " H " + (6.6 * root.u).toFixed(2)
                    + " A " + (2.4 * root.u).toFixed(2) + " " + (2.4 * root.u).toFixed(2) + " 0 0 1 "
                    + (6.6 * root.u).toFixed(2) + " " + (14.4 * root.u).toFixed(2) + " Z"
            }
        }
    }
}
