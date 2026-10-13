/*
 * The CRT over the whole hackergreen popup, drawn once and never animated: one
 * dark device pixel on every third row (scanlines, about 12% on the lit rows)
 * and a radial darkening into the corners (vignette). It takes no input and is
 * drawn only while a hackergreen module holds it (see HgCrt.qml).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import "../../engine"

Item {
    id: crt

    objectName: "hgCrt"
    required property Theme theme
    /*! Modules holding this overlay (HgCrt instances). */
    property int users: 0

    anchors.fill: parent
    z: 1000
    enabled: false
    visible: users > 0 && theme.available

    readonly property real dpr: theme.grid.devicePixelRatio > 0 ? theme.grid.devicePixelRatio : 1
    /*! Device rows per scanline: one dark row, then two clear ones. */
    readonly property int pitchDev: 3
    /*! The scanlines as one path: each row's centre sits on a device pixel. */
    readonly property string scanPath: {
        let d = "";
        const rows = Math.ceil(height * dpr);
        for (let y = 1; y < rows; y += pitchDev) {
            d += "M0 " + ((y + 0.5) / dpr) + " H" + width + " ";
        }
        return d;
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: Qt.rgba(0, 0, 0, 0.12)
            strokeWidth: 1 / crt.dpr
            fillColor: "transparent"
            capStyle: ShapePath.FlatCap

            PathSvg { path: crt.scanPath }
        }
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.GeometryRenderer

        ShapePath {
            strokeColor: "transparent"

            fillGradient: RadialGradient {
                centerX: crt.width / 2
                centerY: crt.height / 2
                focalX: centerX
                focalY: centerY
                centerRadius: Math.sqrt(crt.width * crt.width + crt.height * crt.height) / 2

                GradientStop { position: 0.55; color: Qt.rgba(0, 0, 0, 0) }
                GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.4) }
            }

            PathRectangle { x: 0; y: 0; width: crt.width; height: crt.height }
        }
    }
}
