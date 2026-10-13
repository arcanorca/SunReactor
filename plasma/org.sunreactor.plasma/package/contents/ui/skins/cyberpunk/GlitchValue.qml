/*
 * The hero value with its static glitch: a red copy one step left and a cyan
 * copy one step right, under the yellow figure. The step is whole device pixels.
 * Used on the light cycle's target only.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property CyberStyle style
    property string text: ""
    property font font: style.heroFont
    property color color: style.yellow
    readonly property real offset: style.glitch

    implicitWidth: primary.implicitWidth + offset * 2
    implicitHeight: primary.implicitHeight

    Text {
        id: red

        x: primary.x - root.offset
        y: primary.y
        text: root.text
        textFormat: Text.PlainText
        font: root.font
        color: root.style.red
        opacity: 0.85
    }
    Text {
        id: cyan

        x: primary.x + root.offset
        y: primary.y
        text: root.text
        textFormat: Text.PlainText
        font: root.font
        color: root.style.cyan
        opacity: 0.7
    }
    Text {
        id: primary

        x: root.offset
        text: root.text
        textFormat: Text.PlainText
        font: root.font
        color: root.color
    }
}
