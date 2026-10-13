/*
 * A panel in the style's own window chrome (inside a PixelSurface, art px):
 * Game Boy menu box, PETSCII box, System 1 window, watch LCD, Workbench
 * window, Nothing card or neon chamfer. Children go into the content area:
 * they are parented to an item at (padLeft, padTop) that is `contentWidth`
 * wide; set `contentHeight` to what they need and the panel reports its own
 * implicit height.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "PixelChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property PixelStyle style
    property string title: ""
    property int contentHeight: 0
    default property alias content: inner.data

    readonly property string chrome: style.chrome
    readonly property bool hasTitle: title.length > 0
    readonly property var pads: Chrome.metrics(chrome, hasTitle, titleLabel.artHeight)
    readonly property int padLeft: pads.l
    readonly property int padTop: pads.t
    readonly property int padRight: pads.r
    readonly property int padBottom: pads.b
    readonly property int contentWidth: Math.max(0, width - padLeft - padRight)
    readonly property int titleX: {
        if (chrome === "window" || chrome === "lcd") {
            return Math.floor((width - titleLabel.artWidth) / 2);
        }
        return chrome === "card" ? pads.tx + 6 : pads.tx;
    }

    implicitHeight: padTop + contentHeight + padBottom
    height: implicitHeight

    Repeater {
        model: root.width > 8 && root.height > 8
            ? Chrome.rects(root.chrome, root.width, root.height,
                           { w: root.hasTitle ? titleLabel.artWidth : 0, h: titleLabel.artHeight,
                             x: root.titleX, y: root.pads.ty },
                           { paper: root.style.paper, ink: root.style.ink, frame: root.style.frame,
                             hot: root.style.hot, inverse: root.style.inverse, desk: root.style.desk,
                             light: root.theme.fg, dark: root.theme.bg })
            : []

        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }

    PixelLabel {
        id: titleLabel

        visible: root.hasTitle
        theme: root.theme
        x: root.titleX
        y: root.pads.ty
        text: root.title
        face: root.style.titleFace
        capitals: root.style.capitalTitles
        maximumArtWidth: Math.max(0, root.width - 40)
        color: {
            switch (root.chrome) {
            case "textbox":
            case "petscii":
                return root.style.inverse;
            case "workbench":
                return root.theme.bg;
            case "card":
                return root.style.dim;
            case "lcd":
            case "chamfer":
                return root.style.caption;
            default:
                return root.style.ink;
            }
        }
    }

    Item {
        id: inner

        x: root.padLeft
        y: root.padTop
        width: root.contentWidth
        height: root.contentHeight
    }
}
