/*
 * The XMB wave: three soft translucent ribbons across the popup, over the navy
 * backdrop and under the content. FullRepresentation loads it inside the popup
 * Frame (Modules.OVERLAY). Static: the paths follow the size, nothing animates.
 * One static blur layer softens the edges; it is rendered when the size changes.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Effects
import QtQuick.Shapes

import "../../engine"

Item {
    id: root

    required property Theme theme

    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    clip: true

    /*! The ribbons, top to bottom. `y` holds the top edge's four control heights
        (fractions of the popup height) from left to right; `thick` is the band's
        thickness; `peak` its opacity at the centre (6 to 14 %). */
    readonly property var ribbons: [
        { y: [0.30, 0.06, 0.40, 0.22], thick: 0.22, peak: 0.14 },
        { y: [0.66, 0.46, 0.84, 0.58], thick: 0.14, peak: 0.10 },
        { y: [0.99, 0.84, 1.04, 0.80], thick: 0.08, peak: 0.07 }
    ]

    function ribbonPath(r) {
        const w = root.width;
        const h = root.height;
        const p = (fx, fy) => (fx * w).toFixed(1) + " " + (fy * h).toFixed(1);
        const y = r.y;
        const th = r.thick;
        return "M " + p(-0.1, y[0]) + " C " + p(0.22, y[1]) + " " + p(0.58, y[2]) + " " + p(1.1, y[3])
            + " L " + p(1.1, y[3] + th) + " C " + p(0.58, y[2] + th) + " " + p(0.22, y[1] + th)
            + " " + p(-0.1, y[0] + th) + " Z";
    }

    layer.enabled: true
    layer.smooth: true
    layer.effect: MultiEffect {
        autoPaddingEnabled: false
        blurEnabled: true
        blurMax: 16
        blur: 0.45
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: root.width
                y2: 0
                GradientStop { position: 0; color: root.style.alpha(root.style.accentSoft, root.ribbons[0].peak * 0.4) }
                GradientStop { position: 0.5; color: root.style.alpha(root.style.ink, root.ribbons[0].peak) }
                GradientStop { position: 1; color: root.style.alpha(root.style.accent, root.ribbons[0].peak * 0.5) }
            }
            PathSvg { path: root.ribbonPath(root.ribbons[0]) }
        }

        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: root.width
                y2: 0
                GradientStop { position: 0; color: root.style.alpha(root.style.accent, root.ribbons[1].peak * 0.3) }
                GradientStop { position: 0.5; color: root.style.alpha(root.style.ink, root.ribbons[1].peak) }
                GradientStop { position: 1; color: root.style.alpha(root.style.accentSoft, root.ribbons[1].peak * 0.4) }
            }
            PathSvg { path: root.ribbonPath(root.ribbons[1]) }
        }

        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: root.width
                y2: 0
                GradientStop { position: 0; color: root.style.alpha(root.style.ink, 0) }
                GradientStop { position: 0.5; color: root.style.alpha(root.style.accentSoft, root.ribbons[2].peak) }
                GradientStop { position: 1; color: root.style.alpha(root.style.ink, 0) }
            }
            PathSvg { path: root.ribbonPath(root.ribbons[2]) }
        }
    }
}
