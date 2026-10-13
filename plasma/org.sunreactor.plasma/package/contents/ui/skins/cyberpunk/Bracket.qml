/*
 * A readout in brackets, [39]: the brackets in the muted tone, the value in the
 * ink. One line; the same font for all three parts.
 */

pragma ComponentBehavior: Bound

import QtQuick

Row {
    id: root

    required property CyberStyle style
    property string value: ""
    property font font: style.techFont
    property color color: style.ink
    property color bracketColor: style.muted

    spacing: 0

    Text {
        text: "["
        textFormat: Text.PlainText
        font: root.font
        color: root.bracketColor
    }
    Text {
        text: root.value
        textFormat: Text.PlainText
        font: root.font
        color: root.color
    }
    Text {
        text: "]"
        textFormat: Text.PlainText
        font: root.font
        color: root.bracketColor
    }
}
