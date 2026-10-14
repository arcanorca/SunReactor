/*
 * Panel "readout" mode: the brightness as a number with a level bar under it.
 *
 * Pixel themes: the bitmap font on a small tile of the theme background (a
 * little LCD), all in palette colours, sized in whole art pixels; the largest
 * variant that fits the panel wins (bigger glyphs, border, the % sign), and a
 * panel too small for any tile gets the bare glyphs in the panel's colour.
 * Other renderers: tabular digits in the theme's digit font, panel text
 * colour, the bar in the theme accent (contrast-checked against the panel).
 * Unknown brightness reads "—" and the bar stays empty.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"

Item {
    id: root

    required property PanelView view

    readonly property Theme theme: view.theme
    readonly property bool known: view.percent >= 0
    readonly property string digits: known ? String(view.percent) : "—"
    readonly property real fraction: known ? view.percent / 100 : 0
    readonly property bool pixel: theme.isPixel && theme.bitmapFont.length > 0

    // --- Nothing: the percentage in the dot face, a row of dots under it ------------

    readonly property bool dotted: view.dotted
    /*! Vertical panels are narrow: the face goes tiny and the sign goes, so the digits fit the thickness. */
    readonly property string dotFace: view.vertical ? "tiny" : "small"
    readonly property string dotText: known ? (view.vertical ? digits : digits + "%") : "—"
    readonly property real dotPitch: view.vertical
        ? Math.max(3, Math.floor(view.thickness * view.dpr * 0.9 / 11)) / view.dpr
        : Math.max(3, Math.floor(view.graphicSize * view.dpr * 0.6 / 7)) / view.dpr

    // --- pixel layout: pick the biggest variant that fits ----------------------

    readonly property real unit: theme.grid.unit
    /*! Art px available across and along the panel. */
    readonly property int maxArtH: Math.floor((view.vertical ? Number.MAX_SAFE_INTEGER : view.graphicSize) / unit)
    readonly property int maxArtW: view.vertical ? Math.floor(view.thickness / unit) : 4096
    readonly property var variant: {
        if (!pixel) {
            return null;
        }
        const glyphW = 8;
        const glyphH = theme.bitmapPixelSize;
        const options = [
            { mag: 2, border: 1, pct: true }, { mag: 1, border: 1, pct: true },
            { mag: 1, border: 0, pct: true }, { mag: 1, border: 1, pct: false },
            { mag: 1, border: 0, pct: false },
        ];
        for (const o of options) {
            const text = known && o.pct ? digits + "%" : digits;
            const chars = known ? text.length : 1;
            // border, 1 px padding, glyphs, 1 px gap, 1 px bar, 1 px padding, border
            const w = chars * glyphW * o.mag + 2 + 2 * o.border;
            const h = glyphH * o.mag + 4 + 2 * o.border;
            if (w <= maxArtW && h <= maxArtH) {
                return { mag: o.mag, border: o.border, text: text, w: w, h: h, tile: true };
            }
        }
        return { mag: 1, border: 0, text: digits, w: 0, h: 0, tile: false };
    }

    implicitWidth: dotted ? dotsColumn.implicitWidth
        : (pixel ? (variant.tile ? tile.implicitWidth : bare.implicitWidth) : vector.implicitWidth)
    implicitHeight: dotted ? dotsColumn.implicitHeight
        : (pixel ? (variant.tile ? tile.implicitHeight : bare.implicitHeight) : vector.implicitHeight)

    Column {
        id: dotsColumn

        visible: root.dotted
        spacing: root.dotPitch * 2

        DotText {
            id: dotReadout

            text: root.dotText
            face: root.dotFace
            size: 1
            pitch: root.dotPitch
            litColor: root.view.panelText
            accentColor: root.view.dataInk
            dimColor: root.view.textGhost
        }

        // The level as a row of dots, as wide as the figures. (DotText reports its
        // width through implicitWidth, not through its cols notification.)
        DotBar {
            cols: Math.max(1, Math.round(dotReadout.implicitWidth / root.dotPitch))
            rows: 1
            pitch: root.dotPitch
            value: root.fraction
            thumbAccent: false
            litColor: root.view.panelText
            dimColor: root.view.dotGhost
        }
    }

    PixelSurface {
        id: tile

        visible: root.pixel && root.variant.tile
        grid: root.theme.grid
        artWidth: visible ? root.variant.w : 0
        artHeight: visible ? root.variant.h : 0
        regionName: "panel-readout"

        Rectangle {
            width: tile.artWidth
            height: tile.artHeight
            color: root.view.theme.borderInactive
        }
        Rectangle {
            x: root.variant ? root.variant.border : 0
            y: x
            width: tile.artWidth - 2 * x
            height: tile.artHeight - 2 * x
            color: root.view.theme.bg
        }
        BitmapText {
            x: (root.variant ? root.variant.border : 0) + 1
            y: x
            theme: root.theme
            inSurface: true
            magnification: root.variant ? root.variant.mag : 1
            text: root.variant ? root.variant.text : ""
            color: root.view.tileInk
        }
        // Level bar: whole art pixels, never width * progress.
        Rectangle {
            id: barTrack
            x: (root.variant ? root.variant.border : 0) + 1
            y: tile.artHeight - x - 1
            width: tile.artWidth - 2 * x
            height: 1
            color: root.view.tileDim
        }
        Rectangle {
            x: barTrack.x
            y: barTrack.y
            width: Math.round(barTrack.width * root.fraction)
            height: 1
            color: root.view.tileInk
        }
    }

    BitmapText {
        id: bare

        visible: root.pixel && !root.variant.tile
        theme: root.theme
        text: root.variant ? root.variant.text : ""
        color: root.view.panelText
    }

    Column {
        id: vector

        readonly property real fontSize: {
            const across = root.view.vertical
                ? root.view.thickness * 0.9 / Math.max(2.4, root.digits.length * 0.62 + 0.4)
                : root.view.graphicSize * 0.5;
            return Math.max(9, Math.min(30, Math.round(across)));
        }
        readonly property real barHeight: root.theme.grid.snap(Math.max(2, fontSize * 0.12))

        visible: !root.pixel && !root.dotted
        spacing: Math.max(1, root.theme.grid.snap(fontSize * 0.08))

        Row {
            anchors.horizontalCenter: parent.horizontalCenter
            spacing: 0

            PlasmaComponents3.Label {
                id: number
                text: root.digits
                textFormat: Text.PlainText
                color: root.view.panelText
                font.family: root.theme.digitsFont
                font.pixelSize: vector.fontSize
                font.features: ({ "tnum": 1 })
            }
            PlasmaComponents3.Label {
                anchors.baseline: number.baseline
                visible: root.known
                text: i18ndc(root.view.domain, "Percent sign after a brightness value in the panel", "%")
                textFormat: Text.PlainText
                color: root.view.panelText
                opacity: 0.7
                font.family: root.theme.digitsFont
                font.pixelSize: Math.round(vector.fontSize * 0.62)
            }
        }

        Rectangle {
            width: parent.width
            height: vector.barHeight
            radius: root.theme.isVector ? height / 2 : 0
            color: root.view.trackInk

            Rectangle {
                width: root.theme.grid.snap(parent.width * root.fraction)
                height: parent.height
                radius: parent.radius
                color: root.view.dataInk
            }
        }
    }
}
