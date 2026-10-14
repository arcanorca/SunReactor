/*
 * A section legend as printed on a front panel: tiny caps on the left, a
 * device-pixel hairline running to the right edge, optional caps on the right.
 *
 *   LIGHT CYCLE ──────────────────────────────── 24 H
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property InstrumentStyle style
    property string text: ""
    property string trailing: ""
    property color color: style.inkMuted
    property color ruleColor: style.rule
    /*! Extra items placed after the rule, at the right edge (e.g. tags). */
    default property alias trailingItems: trailingRow.data

    implicitWidth: label.implicitWidth + trailingRow.implicitWidth + style.gap * 4
    implicitHeight: style.snap(Math.max(label.implicitHeight, trailingRow.implicitHeight))

    Text {
        id: label

        anchors.verticalCenter: parent.verticalCenter
        text: root.text
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.color
    }

    Rectangle {
        x: root.style.snap(label.implicitWidth + root.style.gap * 2)
        y: root.style.snap(root.height / 2)
        width: Math.max(0, root.style.snap(trailingRow.x - root.style.gap * 2) - x)
        height: root.style.hairline
        color: root.ruleColor
        visible: width > root.style.gap
    }

    Row {
        id: trailingRow

        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: root.style.gap

        Text {
            visible: root.trailing.length > 0
            anchors.verticalCenter: parent.verticalCenter
            text: root.trailing
            textFormat: Text.PlainText
            font: root.style.legendFont
            color: root.style.inkFaint
        }
    }
}
