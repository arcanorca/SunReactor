/*
 * A Workbench window in art pixels (2.0 look): a 2 px bevel, a grey title bar
 * with the close gadget at left, the depth gadget at right and the title text
 * after the close gadget, a black rule, and a white body. Children go into the
 * body at (padLeft, padTop), contentWidth wide; set contentHeight to what they
 * need and the window reports its own implicit height.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "AmigaChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property AmigaStyle style
    property string title: ""
    property int contentHeight: 0
    /*! No title bar: the bevel and a margin only (the secondary weather panels). */
    property bool plain: false
    default property alias content: inner.data

    readonly property var pads: plain ? Chrome.PLAIN : Chrome.PAD
    readonly property int padLeft: pads.l
    readonly property int padTop: pads.top
    readonly property int padRight: pads.r
    readonly property int padBottom: pads.b
    readonly property int contentWidth: Math.max(0, width - padLeft - padRight)

    implicitHeight: padTop + contentHeight + padBottom
    height: implicitHeight

    Repeater {
        model: root.width > 8 && root.height > 8
            ? (root.plain ? Chrome.plainRects : Chrome.windowRects)(root.width, root.height, {
                  body: root.style.body, face: root.style.face, ink: root.style.ink })
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

    AmigaText {
        id: titleText

        visible: !root.plain && root.title.length > 0
        x: 14
        y: 2
        text: root.title
        color: root.style.ink
        maximumArtWidth: Math.max(0, root.width - 14 - 7 - 8)
    }

    Item {
        id: inner

        x: root.padLeft
        y: root.padTop
        width: root.contentWidth
        height: root.contentHeight
    }
}
