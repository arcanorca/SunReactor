/*
 * Panel "sparkline" mode: today's brightness curve (client.dayCurve, the
 * selected display or the mean) between that display's limits, the past
 * drawn solid, the rest of the day faint, and a dot at the present moment in
 * the state colour.
 *
 *   pixel     one art-pixel line on a tile of the theme background, columns
 *             of whole art pixels, the area under the past shaded;
 *   terminal  a row of block characters (the TUI's sparkline);
 *   vector    an antialiased line with a soft fill.
 * PanelView only picks this mode when there is a curve.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import org.kde.plasma.components as PlasmaComponents3
import org.sunreactor.plasma

import "../engine"
import "PanelData.js" as PanelData

Item {
    id: root

    required property PanelView view

    readonly property Theme theme: view.theme
    readonly property var curve: view.curve
    readonly property real now: view.client.nowEpochS
    readonly property real nowFraction: curve ? PanelData.dayFraction(curve, now) : 0
    readonly property real nowLevel: curve ? PanelData.level(curve, PanelData.valueAt(curve, now)) : 0

    implicitWidth: theme.isPixel ? tile.implicitWidth : (theme.isTerminal ? blocks.implicitWidth : line.width)
    implicitHeight: theme.isPixel ? tile.implicitHeight : (theme.isTerminal ? blocks.implicitHeight : line.height)

    // --- pixel -------------------------------------------------------------------------

    readonly property real unit: theme.grid.unit
    readonly property int artH: view.vertical
        ? Math.max(6, Math.round(view.thickness * 0.55 / unit))
        : Math.max(6, Math.min(16, Math.floor(view.graphicSize / unit)))
    readonly property int artW: view.vertical ? Math.floor(view.thickness / unit) : Math.round(artH * 2.6)
    readonly property int border: artH >= 10 ? 1 : 0
    /*! One entry per inner column: line run, area top, past or future. */
    readonly property var columns: {
        if (!theme.isPixel || !curve) {
            return [];
        }
        const iw = artW - 2 * border - 2;
        const ih = artH - 2 * border - 2;
        const out = [];
        let prev = -1;
        for (let x = 0; x < iw; ++x) {
            const t = curve.t0 + (curve.t1 - curve.t0) * (x + 0.5) / iw;
            const y = ih - 1 - Math.round(PanelData.level(curve, PanelData.valueAt(curve, t)) * (ih - 1));
            const top = prev < 0 ? y : Math.min(y, prev);
            const bottom = prev < 0 ? y : Math.max(y, prev);
            out.push({ x: x, top: top, run: bottom - top + 1, y: y, ih: ih, past: (x + 0.5) / iw <= nowFraction });
            prev = y;
        }
        return out;
    }

    PixelSurface {
        id: tile

        visible: root.theme.isPixel
        grid: root.theme.grid
        artWidth: visible ? root.artW : 0
        artHeight: visible ? root.artH : 0
        regionName: "panel-sparkline"

        Rectangle {
            width: tile.artWidth
            height: tile.artHeight
            color: root.border ? root.theme.borderInactive : root.theme.bg
        }
        Rectangle {
            x: root.border
            y: root.border
            width: tile.artWidth - 2 * root.border
            height: tile.artHeight - 2 * root.border
            color: root.theme.bg
        }
        Repeater {
            model: root.columns
            delegate: Item {
                id: column
                required property var modelData
                x: root.border + 1 + modelData.x
                y: root.border + 1
                width: 1
                height: modelData.ih

                Rectangle {
                    visible: column.modelData.past && column.modelData.y + 1 < column.modelData.ih
                    y: column.modelData.y + 1
                    width: 1
                    height: column.modelData.ih - column.modelData.y - 1
                    color: root.view.tileDim
                }
                Rectangle {
                    y: column.modelData.top
                    width: 1
                    height: column.modelData.run
                    color: column.modelData.past ? root.view.tileInk : root.view.tileDim
                }
            }
        }
        Rectangle {
            readonly property int iw: root.artW - 2 * root.border - 2
            readonly property int ih: root.artH - 2 * root.border - 2
            x: root.border + 1 + Math.min(iw - 2, Math.max(0, Math.round(root.nowFraction * iw) - 1))
            y: root.border + 1 + Math.min(ih - 2, Math.max(0, ih - 1 - Math.round(root.nowLevel * (ih - 1)) - 1))
            width: 2
            height: 2
            color: root.view.tileState
        }
    }

    // --- terminal ----------------------------------------------------------------------

    Row {
        id: blocks

        readonly property real fontSize: root.view.vertical ? 9 : Math.max(9, Math.round(root.view.graphicSize * 0.5))
        readonly property int count: root.view.vertical
            ? Math.max(4, Math.floor(root.view.thickness * 0.92 / (fontSize * 0.6)))
            : 12
        readonly property string glyphs: root.curve && root.theme.isTerminal ? PanelData.blocks(root.curve, count) : ""
        readonly property int split: Math.round(root.nowFraction * count)

        visible: root.theme.isTerminal

        PlasmaComponents3.Label {
            text: blocks.glyphs.slice(0, blocks.split)
            textFormat: Text.PlainText
            color: root.view.dataInk
            font.family: root.theme.monoFont
            font.pixelSize: blocks.fontSize
        }
        PlasmaComponents3.Label {
            text: blocks.glyphs.slice(blocks.split)
            textFormat: Text.PlainText
            color: root.view.trackInk
            font.family: root.theme.monoFont
            font.pixelSize: blocks.fontSize
        }
    }

    // --- vector ------------------------------------------------------------------------

    Item {
        id: line

        readonly property real lw: root.theme.lineWidth * 1.5
        readonly property real pad: dot.width / 2
        readonly property real plotW: width - 2 * pad
        readonly property real plotH: height - 2 * pad

        function pointsBetween(from, to, closeArea) {
            const out = [];
            if (!root.curve) {
                return out;
            }
            for (const p of root.curve.points) {
                const f = PanelData.dayFraction(root.curve, p.t);
                if (f < from || f > to) {
                    continue;
                }
                out.push(Qt.point(pad + f * plotW, pad + (1 - PanelData.level(root.curve, p.pct)) * plotH));
            }
            // The split point itself, so both halves meet at the present.
            const nowPoint = Qt.point(pad + root.nowFraction * plotW, pad + (1 - root.nowLevel) * plotH);
            if (to === root.nowFraction) {
                out.push(nowPoint);
            } else if (from === root.nowFraction) {
                out.unshift(nowPoint);
            }
            if (closeArea && out.length > 0) {
                out.push(Qt.point(out[out.length - 1].x, pad + plotH));
                out.push(Qt.point(out[0].x, pad + plotH));
            }
            return out;
        }

        visible: !root.theme.isPixel && !root.theme.isTerminal
        height: root.theme.grid.snap(root.view.vertical ? root.view.thickness * 0.5 : root.view.graphicSize * 0.72)
        width: root.theme.grid.snap(root.view.vertical ? root.view.thickness * 0.86 : height * 2.6)

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeWidth: -1
                strokeColor: "transparent"
                fillColor: Qt.rgba(root.view.dataInk.r, root.view.dataInk.g, root.view.dataInk.b, 0.2)
                PathPolyline {
                    path: line.visible ? line.pointsBetween(0, root.nowFraction, true) : []
                }
            }
            ShapePath {
                strokeWidth: line.lw
                strokeColor: root.view.trackInk
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                PathPolyline {
                    path: line.visible ? line.pointsBetween(root.nowFraction, 1, false) : []
                }
            }
            ShapePath {
                strokeWidth: line.lw
                strokeColor: root.view.dataInk
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                joinStyle: ShapePath.RoundJoin
                PathPolyline {
                    path: line.visible ? line.pointsBetween(0, root.nowFraction, false) : []
                }
            }
        }

        Rectangle {
            id: dot

            width: root.theme.grid.snap(Math.max(5, line.height * 0.24))
            height: width
            radius: width / 2
            x: line.pad + root.nowFraction * line.plotW - width / 2
            y: line.pad + (1 - root.nowLevel) * line.plotH - height / 2
            color: root.view.stateInk
            border.width: root.theme.lineWidth
            border.color: root.view.panelBackground
        }
    }
}
