/*
 * The theme button's glyph, Nothing, drawn inside its round hairline (ThemeButton
 * owns the circle, the hover and the focus). Four dots on the corners of a 3 x 3
 * matrix show the active palette: ink, secondary accent, the accent and the
 * muted grey. A dot on the top-right edge is the badge: solid ink while following
 * the terminal app, a hollow ring when pinned, and the one red dot when the
 * daemon fell back to another theme.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

Item {
    id: root

    required property Theme theme
    property bool pinned: false
    property bool warned: false

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    /*! Dot pitch of the 3 x 3 matrix: the four dots sit 14 px apart in a 24 px circle. */
    readonly property real pitch: 7

    anchors.fill: parent

    DotField {
        id: swatch

        x: root.style.snap((root.width - width) / 2)
        y: root.style.snap((root.height - height) / 2)
        cols: 3
        rows: 3
        pitch: root.pitch
        dotScale: 0.7
        litColor: root.theme.fg
        ghostColor: root.theme.secondaryAccent
        accentColor: root.theme.accent
        dimColor: root.theme.textMuted

        Component.onCompleted: {
            swatch.setCell(0, 0, DotField.Lit);
            swatch.setCell(2, 0, DotField.Ghost);
            swatch.setCell(0, 2, DotField.Accent);
            swatch.setCell(2, 2, DotField.Off);
        }
    }

    // The badge: centred on the circle's edge at 45 degrees. Its background-coloured
    // ring cuts the hairline, so the dot reads over the outline.
    readonly property real badgeSize: root.style.snap(6)
    readonly property real badgeX: root.style.snap(root.width / 2 + root.width / 2 * Math.SQRT1_2 - badgeSize / 2)
    readonly property real badgeY: root.style.snap(root.height / 2 - root.height / 2 * Math.SQRT1_2 - badgeSize / 2)

    Rectangle {
        x: root.badgeX - root.style.snap(2)
        y: root.badgeY - root.style.snap(2)
        width: root.badgeSize + root.style.snap(4)
        height: width
        radius: width / 2
        antialiasing: true
        color: root.theme.bg
    }

    Rectangle {
        x: root.badgeX
        y: root.badgeY
        width: root.badgeSize
        height: width
        radius: width / 2
        antialiasing: true
        // Following: solid ink. Pinned: a hollow ring. Warned: the one red dot.
        color: root.warned ? root.style.accent : (root.pinned ? "transparent" : root.style.ink)
        border.width: root.pinned && !root.warned ? root.style.hairline * 2 : 0
        border.color: root.style.ink
    }
}
