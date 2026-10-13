/*
 * A channel tile: a large rounded face with a thin cool-grey outline, a soft
 * gloss over its top 40% and a faint static shadow. `lit` draws the selected
 * outline, 2 px Wii blue with one flat halo ring (no blur, no animation).
 *
 * The caller sizes the tile (implicitHeight from its content) and puts its
 * layout inside, anchored to the tile's padding: `content` is the padded box.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    required property WiiStyle style
    /*! The selected state: the Wii blue outline (a hovered tile lights too). */
    property bool lit: false
    /*! The pointer is over the tile. A passive handler: it never takes a click. */
    readonly property bool hovered: hover.hovered
    /*! Padding between the outline and the content. */
    property real pad: style.tilePad
    readonly property real tileRadius: style.radiusFor(width, height)

    /*! The content box: inset by the padding on every side. */
    default property alias content: body.data

    // Shadow: two faint offsets below the face, so the tile sits on the page.
    Rectangle {
        x: 0
        y: root.style.snap(3)
        width: root.width
        height: root.height
        radius: root.tileRadius
        antialiasing: true
        color: root.style.alpha(root.style.ink, 0.05)
    }
    Rectangle {
        x: 0
        y: root.style.snap(1.5)
        width: root.width
        height: root.height
        radius: root.tileRadius
        antialiasing: true
        color: root.style.alpha(root.style.ink, 0.06)
    }

    Rectangle {
        id: face

        anchors.fill: parent
        radius: root.tileRadius
        antialiasing: true
        color: root.style.face
        border.width: root.style.line
        border.color: root.style.outline
    }

    // The gloss: white fading out over the top 40% of the face.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.margins: root.style.line
        height: Math.round(root.height * 0.4)
        radius: Math.max(0, root.tileRadius - root.style.line)
        antialiasing: true
        gradient: Gradient {
            GradientStop { position: 0; color: root.style.gloss }
            GradientStop { position: 1; color: root.style.glossClear }
        }
    }

    HoverHandler {
        id: hover
    }

    // The selected outline: a halo ring, then the 2 px blue edge.
    Rectangle {
        visible: root.lit || root.hovered
        anchors.fill: parent
        anchors.margins: -root.style.line2 * 2
        radius: root.tileRadius + root.style.line2 * 2
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.alpha(root.style.accent, 0.3)
    }
    Rectangle {
        visible: root.lit || root.hovered
        anchors.fill: parent
        radius: root.tileRadius
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.accent
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: root.pad
    }
}
