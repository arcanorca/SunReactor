/*
 * A rounded pill button, the Wii menu's bottom-corner buttons and OK / Back
 * pills. Two variants:
 *
 *   "fill"     the mode buttons: the chosen one is filled deep Wii blue with
 *              white text; the others are light, outlined in grey. `called`
 *              gives an unchosen one the blue outline (the way back).
 *   "outline"  the page tabs: a white pill, outlined in blue when chosen.
 *
 * `hint` is a second, smaller text after the label; `arrow` adds a chevron.
 * The owner sets the accessible role and handles `clicked`.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property WiiStyle style
    property string label: ""
    property string hint: ""
    property bool current: false
    /*! The way back: an unchosen "fill" pill gets the blue outline. */
    property bool called: false
    property string variant: "fill"
    property bool arrow: false
    /*! Keyboard focus: a thin blue ring outside the pill. */
    property bool focused: false
    readonly property bool hovered: mouse.containsMouse

    signal clicked()
    /*! The chevron was pressed: a separate target, for the pause durations. */
    signal arrowClicked()

    readonly property bool filled: variant === "fill" && current
    readonly property real radius: height / 2
    readonly property color textColor: filled ? style.paper
        : (variant === "fill" ? style.ink : (current ? style.ink : style.inkSoft))

    implicitWidth: row.implicitWidth + 2 * style.gap2 * 1.5
    implicitHeight: style.pillHeight
    opacity: enabled ? 1 : 0.5

    // A faint shadow under the pill, then the face.
    Rectangle {
        x: 0
        y: root.style.snap(1.5)
        width: root.width
        height: root.height
        radius: root.radius
        antialiasing: true
        color: root.style.alpha(root.style.ink, 0.06)
        visible: !root.filled
    }
    Rectangle {
        id: face

        anchors.fill: parent
        radius: root.radius
        antialiasing: true
        color: root.filled ? root.style.accentInk
            : (root.variant === "outline" && root.current ? root.style.paper : root.style.face)
        border.width: root.style.line
        border.color: root.filled ? root.style.accentInk
            : (root.current || root.called ? root.style.accentLine : root.style.outline)
    }
    // The chosen outline or the way back: a 2 px blue edge.
    Rectangle {
        visible: !root.filled && (root.current || root.called)
        anchors.fill: parent
        radius: root.radius
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.accentLine
    }
    Rectangle {
        visible: root.focused
        x: -root.style.line2 * 2
        y: -root.style.line2 * 2
        width: root.width + root.style.line2 * 4
        height: root.height + root.style.line2 * 4
        radius: root.radius + root.style.line2 * 2
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.accent
    }
    // Hover: a light blue wash over the face.
    Rectangle {
        visible: root.hovered && !root.filled
        anchors.fill: parent
        radius: root.radius
        antialiasing: true
        color: root.style.alpha(root.style.accent, 0.1)
    }

    Row {
        id: row

        anchors.centerIn: parent
        spacing: root.style.gap

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.label
            textFormat: Text.PlainText
            font: root.style.bodyBoldFont
            color: root.textColor
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: text.length > 0
            text: root.hint
            textFormat: Text.PlainText
            font: root.style.captionFont
            color: root.filled ? root.style.paper : root.style.inkSoft
        }
        Kirigami.Icon {
            id: chevron

            visible: root.arrow
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(root.style.unit * 0.6)
            height: width
            source: "arrow-down"
            isMask: true
            color: root.textColor
        }
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
    // The chevron's own target, drawn over the pill so it takes its clicks first.
    MouseArea {
        visible: root.arrow
        x: chevron.x - root.style.gap
        y: 0
        width: chevron.width + 2 * root.style.gap
        height: root.height
        cursorShape: Qt.PointingHandCursor
        onClicked: root.arrowClicked()
    }
}
