/*
 * Nintendo DS: a glossy rounded DS-blue tile with a white sheen on its upper half,
 * and the solid glyph in the DS colours (cream cloud, orange sun, light-blue rain,
 * yellow bolt). The palette is the handheld's own, not the pixel sprite.
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
    readonly property color blueTop: "#6aa8f2"
    readonly property color blueBottom: "#1f4ea6"

    Rectangle {
        id: tile

        width: root.tileSide
        height: root.tileSide
        anchors.centerIn: parent
        radius: Math.round(root.tileSide * 0.32)
        antialiasing: true
        border.width: 1
        border.color: "#1e4390"
        gradient: Gradient {
            GradientStop { position: 0; color: root.blueTop }
            GradientStop { position: 1; color: root.blueBottom }
        }
    }

    Rectangle {
        // The gloss: a white sheen over the upper half of the tile.
        x: tile.x + 1
        y: tile.y + 1
        width: tile.width - 2
        height: Math.round(tile.height * 0.48)
        radius: tile.radius - 1
        antialiasing: true
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.5) }
            GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
        }
    }

    WeatherGlyph {
        anchors.centerIn: parent
        condition: root.condition
        night: root.night
        glyphStyle: "solid"
        palette: ({
            bg: root.theme.hexOf(root.blueBottom),
            fg: "#fff4dc",
            text_muted: "#d5e6fa",
            accent: "#ff9d2e",
            secondary_accent: "#a8d8ff",
            warning: "#ffd45c"
        })
        size: Math.round(root.size * 0.72)
    }
}
