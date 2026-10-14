/*
 * Windows XP (Luna): a glossy blue tile, rounded, with a white highlight on its
 * upper half, and the solid glyph in the Luna colours (gold sun, white cloud).
 * The tile follows the theme's accent, so a Luna variant keeps its blue.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    property real size: 22
    property color surface: theme.bg
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    readonly property real dpr: theme.grid.devicePixelRatio
    readonly property real tileSide: Math.round(size * dpr) / dpr
    readonly property color tileTop: theme.mix(theme.accent, Qt.rgba(1, 1, 1, 1), 0.35)
    readonly property color tileBottom: theme.mix(theme.accent, Qt.rgba(0, 0, 0, 1), 0.18)

    Rectangle {
        id: tile

        width: root.tileSide
        height: root.tileSide
        anchors.centerIn: parent
        radius: Math.round(root.tileSide * 0.14)
        antialiasing: true
        border.width: 1
        border.color: root.theme.borderActive
        gradient: Gradient {
            GradientStop { position: 0; color: root.tileTop }
            GradientStop { position: 1; color: root.tileBottom }
        }
    }

    Rectangle {
        // The Luna rim: a pale line inside the tile's edge.
        x: tile.x + 2
        y: tile.y + 2
        width: tile.width - 4
        height: tile.height - 4
        radius: tile.radius - 2
        color: "transparent"
        border.width: 1
        border.color: Qt.rgba(1, 1, 1, 0.55)
        antialiasing: true
    }

    Rectangle {
        // The Luna gloss: a white sheen over the upper half of the tile.
        x: tile.x + 1
        y: tile.y + 1
        width: tile.width - 2
        height: Math.round(tile.height * 0.5)
        radius: tile.radius - 1
        antialiasing: true
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.45) }
            GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
        }
    }

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "solid"
        palette: ({
            bg: root.theme.hexOf(root.tileBottom),
            fg: "#ffffff",
            text_muted: "#dbe7ff",
            accent: root.theme.hexOf(root.theme.warning),
            secondary_accent: "#9fd8ff",
            warning: root.theme.hexOf(root.theme.warning)
        })
        size: Math.round(root.size * 0.7)
    }
}
