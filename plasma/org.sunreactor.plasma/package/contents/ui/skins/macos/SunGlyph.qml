/*
 * The sun of a brightness slider: a disc with eight rays, drawn as one path in
 * one colour. Static: it is built once from `size` and never animates.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    /*! The whole glyph, logical px. */
    property real size: 14
    property color color: "transparent"

    readonly property real centre: size / 2
    readonly property real discRadius: size * 0.2
    readonly property real rayInner: size * 0.36
    readonly property real rayOuter: size * 0.5

    /*! The disc and the rays as one SVG path (PathSvg takes it as one string). */
    readonly property string outline: {
        const c = centre;
        const d = discRadius;
        let path = "M " + (c - d) + " " + c + " A " + d + " " + d + " 0 1 0 " + (c + d) + " " + c
            + " A " + d + " " + d + " 0 1 0 " + (c - d) + " " + c;
        for (let i = 0; i < 8; ++i) {
            const a = i * Math.PI / 4;
            const cos = Math.cos(a);
            const sin = Math.sin(a);
            path += " M " + (c + rayInner * cos) + " " + (c + rayInner * sin)
                + " L " + (c + rayOuter * cos) + " " + (c + rayOuter * sin);
        }
        return path;
    }

    implicitWidth: size
    implicitHeight: size
    width: size
    height: size

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.color
            strokeWidth: Math.max(1.25, root.size * 0.09)
            fillColor: root.color
            capStyle: ShapePath.RoundCap

            PathSvg {
                path: root.outline
            }
        }
    }
}
