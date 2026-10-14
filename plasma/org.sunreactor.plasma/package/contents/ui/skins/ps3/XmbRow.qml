/*
 * One detail line of an XMB list: the name on the left in sub-text, its value
 * on the right in light tabular figures, and a faint separator below.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property XmbStyle style
    property string label: ""
    property string value: ""
    /*! Space above the text, and below it before the separator. */
    property real padding: style.gap

    Layout.fillWidth: true
    implicitWidth: 160
    implicitHeight: line.implicitHeight + padding * 2

    RowLayout {
        id: line

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: root.padding

        Text {
            Layout.fillWidth: true
            text: root.label
            textFormat: Text.PlainText
            font: root.style.labelFont
            color: root.style.inkSoft
            lineHeightMode: Text.FixedHeight
            lineHeight: root.style.sizeBody
            elide: Text.ElideRight
        }

        Text {
            text: root.value
            textFormat: Text.PlainText
            horizontalAlignment: Text.AlignRight
            font: root.style.valueFont
            color: root.style.ink
            lineHeightMode: Text.FixedHeight
            lineHeight: root.style.sizeBody
        }
    }

    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: root.style.hairline
        color: root.style.rule
    }
}
