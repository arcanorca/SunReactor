/*
 * A readout in brackets, [39]: the brackets in the muted tone, the value in the
 * ink. One line; the same font for all three parts.
 *
 * The size hint is ceiled to whole device pixels. Three Texts sum to a
 * fractional width (63.59 at 1.35x is 85.85 device pixels); a layout that
 * snaps its allocation to pixels then disagrees with the hint, and the page
 * polishes forever, one pixel back and forth per frame.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property CyberStyle style
    property string value: ""
    property font font: style.techFont
    property color color: style.ink
    property color bracketColor: style.muted

    readonly property real dpr: Math.max(1, Screen.devicePixelRatio)

    implicitWidth: Math.ceil(row.implicitWidth * dpr - 0.001) / dpr
    implicitHeight: Math.ceil(row.implicitHeight * dpr - 0.001) / dpr

    Row {
        id: row

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
}
