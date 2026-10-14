/*
 * A card: 6 px rounded, a 1 device-px neon outline (pink or cyan), a dark body
 * and a soft inner glow (a faint neon band just inside the outline). No chrome.
 * Children go into the card's column.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property SynthStyle style
    /*! Outline and inner glow colour. */
    property color neon: style.cyan
    default property alias content: column.data

    implicitWidth: column.implicitWidth + 2 * root.style.pad
    implicitHeight: column.implicitHeight + 2 * root.style.pad

    // The body and the outline.
    Rectangle {
        anchors.fill: parent
        radius: root.style.radius
        antialiasing: true
        color: root.style.card
        border.width: root.style.hairline
        border.color: root.neon
    }

    // Soft inner glow: a wide, faint band just inside the outline.
    Rectangle {
        anchors.fill: parent
        anchors.margins: root.style.snap(root.style.gap * 0.6)
        radius: Math.max(0, root.style.radius - 2)
        antialiasing: true
        color: "transparent"
        border.width: root.style.snap(root.style.gap * 1.2)
        border.color: root.style.alpha(root.neon, 0.07)
    }

    ColumnLayout {
        id: column

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: root.style.pad
        spacing: root.style.gap
    }
}
