/*
 * Panel "ring" mode: a 270 degree gauge of the brightness, open at the
 * bottom, filled in the daemon's state colour (live / paused or manual /
 * offline), with the number inside when there is room.
 *
 *   pixel     a ring of whole art pixels on a tile of the theme background,
 *             the number in a 3x5 pixel figure set (the bitmap font is too
 *             big to fit inside a panel-sized ring);
 *   terminal  segmented arc, square ends;
 *   vector    continuous arc, round ends.
 * Unknown brightness leaves the arc empty and reads "—".
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.plasma.components as PlasmaComponents3
import org.sunreactor.plasma

import "../engine"

Item {
    id: root

    required property PanelView view

    readonly property Theme theme: view.theme
    readonly property bool known: view.percent >= 0
    readonly property real fraction: known ? view.percent / 100 : 0

    implicitWidth: dotted ? ringDots.implicitWidth : (theme.isPixel ? tile.implicitWidth : gauge.width)
    implicitHeight: dotted ? ringDots.implicitHeight : (theme.isPixel ? tile.implicitHeight : gauge.height)

    // --- Nothing: a dotted circle with the figure inside -------------------------------

    readonly property bool dotted: view.dotted
    /*! Thirteen dots across: the figure fits the hole, and the ring a whole device pixel per dot. */
    readonly property real dotPitch: Math.max(3, Math.floor(view.graphicSize * view.dpr / 13)) / view.dpr

    DotRing {
        id: ringDots

        visible: root.dotted
        diameter: 13
        dots: 36
        value: root.fraction
        headAccent: root.known
        pitch: root.dotPitch
        litColor: root.view.panelText
        accentColor: root.view.dataInk
        dimColor: root.view.dotGhost
    }

    DotText {
        anchors.centerIn: parent
        visible: root.dotted
        text: root.known ? String(root.view.percent) : "—"
        face: "tiny"
        size: 1
        letterSpacing: 0
        pitch: root.dotPitch
        litColor: root.view.panelText
        dimColor: "transparent"
    }

    // --- pixel -------------------------------------------------------------------------

    readonly property int artD: Math.max(9, Math.floor(view.graphicSize / theme.grid.unit))
    readonly property int stroke: artD >= 16 ? 2 : 1
    readonly property var cells: {
        if (!theme.isPixel) {
            return [];
        }
        const c = (artD - 1) / 2;
        const outer = artD / 2 - 1;
        const out = [];
        for (let y = 0; y < artD; ++y) {
            for (let x = 0; x < artD; ++x) {
                const d = Math.hypot(x - c, y - c);
                if (d > outer + 0.25 || d <= outer + 0.25 - stroke) {
                    continue;
                }
                const angle = Math.atan2(y - c, x - c) * 180 / Math.PI;
                const along = ((angle - 135) % 360 + 360) % 360;
                if (along > 270) {
                    continue;
                }
                out.push({ x: x, y: y, on: known && along <= 270 * fraction + 0.5 });
            }
        }
        return out;
    }

    // 3x5 figures, row-major bits.
    readonly property var figures: ({
        "0": "111101101101111", "1": "010110010010111", "2": "111001111100111",
        "3": "111001111001111", "4": "101101111001001", "5": "111100111001111",
        "6": "111100111101111", "7": "111001001010010", "8": "111101111101111",
        "9": "111101111001111", "-": "000000111000000",
    })
    readonly property string figureText: known ? String(view.percent) : "-"
    readonly property int figureW: figureText.length * 4 - 1
    readonly property bool figuresFit: figureW <= artD - 2 * stroke - 4 && artD - 2 * stroke - 4 >= 5
    readonly property var figurePixels: {
        if (!theme.isPixel || !figuresFit) {
            return [];
        }
        const ox = Math.floor((artD - figureW) / 2);
        const oy = Math.floor((artD - 5) / 2);
        const out = [];
        for (let i = 0; i < figureText.length; ++i) {
            const bits = figures[figureText[i]] || figures["-"];
            for (let b = 0; b < 15; ++b) {
                if (bits[b] === "1") {
                    out.push({ x: ox + i * 4 + b % 3, y: oy + Math.floor(b / 3) });
                }
            }
        }
        return out;
    }

    PixelSurface {
        id: tile

        visible: root.theme.isPixel
        grid: root.theme.grid
        artWidth: visible ? root.artD : 0
        artHeight: visible ? root.artD : 0
        regionName: "panel-ring"

        Rectangle {
            width: tile.artWidth
            height: tile.artHeight
            color: root.theme.bg
        }
        Repeater {
            model: root.cells
            delegate: Rectangle {
                required property var modelData
                x: modelData.x
                y: modelData.y
                width: 1
                height: 1
                color: modelData.on ? root.view.tileState : root.view.tileDim
            }
        }
        Repeater {
            model: root.figurePixels
            delegate: Rectangle {
                required property var modelData
                x: modelData.x
                y: modelData.y
                width: 1
                height: 1
                color: root.view.tileInk
            }
        }
    }

    // --- vector / terminal -------------------------------------------------------------

    Item {
        id: gauge

        readonly property real stroke: root.theme.grid.snap(Math.max(2, width * (root.theme.isTerminal ? 0.14 : 0.1)))
        readonly property real radius: (width - stroke) / 2

        visible: !root.theme.isPixel && !root.dotted
        width: root.theme.grid.snap(root.view.graphicSize)
        height: width

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: gauge.stroke
                strokeColor: root.view.trackInk
                fillColor: "transparent"
                capStyle: root.theme.isTerminal ? ShapePath.FlatCap : ShapePath.RoundCap
                strokeStyle: root.theme.isTerminal ? ShapePath.DashLine : ShapePath.SolidLine
                dashPattern: [1.1, 0.45]
                PathAngleArc {
                    centerX: gauge.width / 2
                    centerY: gauge.height / 2
                    radiusX: gauge.radius
                    radiusY: gauge.radius
                    startAngle: 135
                    sweepAngle: 270
                }
            }
            ShapePath {
                strokeWidth: gauge.stroke
                strokeColor: root.known && root.fraction > 0 ? root.view.stateInk : "transparent"
                fillColor: "transparent"
                capStyle: root.theme.isTerminal ? ShapePath.FlatCap : ShapePath.RoundCap
                strokeStyle: root.theme.isTerminal ? ShapePath.DashLine : ShapePath.SolidLine
                dashPattern: [1.1, 0.45]
                PathAngleArc {
                    centerX: gauge.width / 2
                    centerY: gauge.height / 2
                    radiusX: gauge.radius
                    radiusY: gauge.radius
                    startAngle: 135
                    sweepAngle: Math.max(0.01, 270 * root.fraction)
                }
            }
        }

        PlasmaComponents3.Label {
            anchors.centerIn: parent
            visible: gauge.width >= 26
            text: root.known ? String(root.view.percent) : "—"
            textFormat: Text.PlainText
            color: root.view.panelText
            font.family: root.theme.digitsFont
            font.features: ({ "tnum": 1 })
            font.pixelSize: Math.max(8, Math.round(gauge.width * (root.view.percent >= 100 ? 0.27 : 0.33)))
        }
    }
}
