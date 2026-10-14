/*
 * An XMB text item: a word that is bright, with a short underline and a soft
 * glow under it, when selected; dim otherwise. A focus scope, so a parent can
 * give it the keyboard: Left and Right ask for a neighbour (`moved`), Space,
 * Enter or a click pick it (`activated`), Down asks for its menu (`down`). With
 * `arrow`, a small chevron after the word opens that menu on a click.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

FocusScope {
    id: root

    required property XmbStyle style
    property string text: ""
    property bool selected: false
    /*! Shows a chevron after the word that opens a menu (the pause durations). */
    property bool arrow: false
    property font textFont: style.labelFont

    signal activated()
    signal moved(int delta)
    signal down()
    signal arrowActivated()

    implicitWidth: label.implicitWidth + (arrow ? chevron.width + style.gap : 0)
    implicitHeight: label.implicitHeight + style.gap2
    activeFocusOnTab: true

    Keys.onLeftPressed: root.moved(-1)
    Keys.onRightPressed: root.moved(1)
    Keys.onDownPressed: root.down()
    Keys.onSpacePressed: root.activated()
    Keys.onReturnPressed: root.activated()
    Keys.onEnterPressed: root.activated()

    Text {
        id: label

        anchors.left: parent.left
        anchors.top: parent.top
        anchors.topMargin: root.style.gap
        text: root.text
        textFormat: Text.PlainText
        font: root.textFont
        color: root.selected || root.activeFocus || hover.containsMouse ? root.style.ink : root.style.inkDim
    }

    // The underline: a soft glow with a bright hairline on top, under the selected word.
    Rectangle {
        visible: root.selected
        anchors.left: label.left
        anchors.right: label.right
        anchors.top: label.bottom
        anchors.topMargin: root.style.gap
        height: root.style.snap(root.style.line2 * 3)
        radius: height / 2
        color: root.style.alpha(root.style.ink, 0.18)
    }
    Rectangle {
        visible: root.selected || root.activeFocus
        anchors.left: label.left
        anchors.right: label.right
        anchors.top: label.bottom
        anchors.topMargin: root.style.gap
        height: root.style.line2
        color: root.selected ? root.style.ink : root.style.alpha(root.style.ink, 0.45)
    }

    Shape {
        id: chevron

        visible: root.arrow
        anchors.left: label.right
        anchors.leftMargin: root.style.gap
        anchors.verticalCenter: label.verticalCenter
        width: root.style.snap(root.style.unit * 0.55)
        height: width
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            fillColor: "transparent"
            strokeColor: root.selected || root.activeFocus || hover.containsMouse ? root.style.ink : root.style.inkDim
            strokeWidth: root.style.line2
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: "M 1 " + (chevron.height * 0.35).toFixed(2) + " L " + (chevron.width / 2).toFixed(2) + " "
                + (chevron.height * 0.7).toFixed(2) + " L " + (chevron.width - 1).toFixed(2) + " "
                + (chevron.height * 0.35).toFixed(2) }
        }
    }

    // The arrow has its own hit area: a click there opens the menu, the word picks.
    MouseArea {
        id: arrowMouse

        visible: root.arrow
        anchors.fill: chevron
        anchors.margins: -root.style.gap
        cursorShape: Qt.PointingHandCursor
        onClicked: root.arrowActivated()
    }

    MouseArea {
        id: hover

        anchors.left: parent.left
        anchors.top: parent.top
        width: label.implicitWidth
        height: parent.height
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.activated();
        }
    }
}
