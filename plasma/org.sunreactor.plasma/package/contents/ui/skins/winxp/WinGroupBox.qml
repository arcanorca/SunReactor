/*
 * An XP group box: a 1 px etched frame (a dark edge over a white one) with the
 * caption in the Luna blue, set on the frame's top edge.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property WinStyle style
    property string caption: ""
    default property alias content: body.data

    readonly property real capH: style.snap(style.sizeBody * 1.3)

    implicitHeight: body.y + body.implicitHeight + style.pad

    Rectangle {
        x: 0
        y: root.capH / 2
        width: root.width
        height: root.height - root.capH / 2
        radius: 3
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.etchDark
    }
    Rectangle {
        x: root.style.line
        y: root.capH / 2 + root.style.line
        width: root.width - 2 * root.style.line
        height: root.height - root.capH / 2 - 2 * root.style.line
        radius: 2
        color: "transparent"
        border.width: root.style.line
        border.color: root.style.etchLight
    }

    // The caption sits on the client colour, over the frame's top edge.
    Rectangle {
        x: root.style.pad - root.style.gap
        y: 0
        width: captionText.implicitWidth + root.style.gap * 2
        height: root.capH
        color: root.style.panel
    }
    Text {
        id: captionText

        x: root.style.pad
        y: (root.capH - height) / 2
        text: root.caption
        textFormat: Text.PlainText
        font: root.style.boldFont
        color: root.style.frame
    }

    ColumnLayout {
        id: body

        x: root.style.pad
        y: root.capH + root.style.gap
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap
    }
}
