/*
 * A chamfered HUD panel: navy fill, a faint diagonal hatch, a 1.5 device px
 * yellow outline, the top-right and bottom-left corners cut. A titled panel
 * carries a solid yellow tab on its top-right cut, the title in condensed caps.
 * Children go into the padded column (the default property).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes

import "Geometry.js" as G

Item {
    id: root

    required property CyberStyle style
    /*! The tab's title; "" draws a plain panel. */
    property string title: ""
    /*! Small technical readout on the tab's line, at the left. */
    property string tag: ""
    property bool hatched: true
    /*! Outline colour. */
    property color edge: style.yellow

    /*! Inset from the outline to the content, on the sides and at the bottom. */
    property real inset: style.pad
    /*! Gap between the rows of the content. */
    property real contentSpacing: style.rowGap
    /*! A titled panel is cut as deep as its tab is high, so the tab edge is the cut. */
    readonly property real cut: title.length > 0 ? style.tabHeight : style.cut
    readonly property real topInset: title.length > 0 ? style.tabHeight + style.gap : inset
    /*! An untitled panel is cut at its top-right and bottom-left: its rows keep clear of the cut. */
    readonly property real sideInset: inset + (title.length > 0 ? 0 : cut * 0.5)

    default property alias content: body.data

    implicitWidth: body.implicitWidth + sideInset * 2
    implicitHeight: topInset + body.implicitHeight + inset

    Accessible.role: Accessible.Grouping
    Accessible.name: root.title

    // Fill.
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeWidth: -1
            fillColor: root.style.panel
            PathSvg { path: G.chamfer(root.width, root.height, root.cut, 0) }
        }
    }

    // Hatch: cheap diagonal lines, inside the box only.
    Shape {
        anchors.fill: parent
        visible: root.hatched
        ShapePath {
            strokeColor: root.style.hatch
            strokeWidth: root.style.hairline
            fillColor: "transparent"
            PathSvg { path: G.hatch(root.width, root.height, root.cut, root.style.hatchStep) }
        }
    }

    // Outline, drawn over the hatch.
    Shape {
        anchors.fill: parent
        ShapePath {
            strokeColor: root.edge
            strokeWidth: root.style.outline
            fillColor: "transparent"
            joinStyle: ShapePath.MiterJoin
            PathSvg { path: G.chamfer(root.width, root.height, root.cut, root.style.outline / 2) }
        }
    }

    // Title tab, hanging from the top edge against the top-right cut.
    Item {
        id: tab

        visible: root.title.length > 0
        width: titleText.implicitWidth + root.style.pad * 2 + root.cut
        height: root.cut
        x: root.width - width
        y: 0

        Shape {
            anchors.fill: parent
            ShapePath {
                strokeWidth: -1
                fillColor: root.style.yellow
                PathSvg { path: G.tab(tab.width, tab.height) }
            }
        }
        Text {
            id: titleText

            x: root.style.pad
            anchors.verticalCenter: parent.verticalCenter
            text: root.title
            textFormat: Text.PlainText
            font: root.style.tabFont
            color: root.style.onYellow
        }
    }

    Text {
        visible: root.tag.length > 0
        x: root.style.pad
        y: (root.cut - implicitHeight) / 2
        text: root.tag
        textFormat: Text.PlainText
        font: root.style.tinyFont
        color: root.style.muted
    }

    ColumnLayout {
        id: body

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: root.sideInset
        anchors.rightMargin: root.sideInset
        anchors.topMargin: root.topInset
        spacing: root.contentSpacing
    }
}
