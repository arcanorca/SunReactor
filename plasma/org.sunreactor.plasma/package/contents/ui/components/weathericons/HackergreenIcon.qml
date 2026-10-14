/*
 * hackergreen: the three-row ASCII sky of the popup (Hg.js), in the phosphor of
 * its condition: the accent for a clear sky, the muted ink for mist, the ink
 * otherwise. Below 9 device px of font the rows are too small to read, and the generic
 * line glyph is drawn instead.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../engine"
import "../../skins/hackergreen/Hg.js" as Hg

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

    /*! Font size of the rows: seven cells of about 0.6 em across fit the square. */
    readonly property int fontPx: Math.floor(size / 4.2)
    /*! The rows need at least 9 device px of font to read. */
    readonly property bool legible: fontPx * theme.grid.devicePixelRatio >= 9
    readonly property color artColour: {
        if (condition === "clear") {
            return theme.accent;
        }
        if (condition === "mist" || condition === "fog" || condition === "atmospheric") {
            return theme.textMuted;
        }
        return ink;
    }

    Column {
        anchors.centerIn: parent
        visible: root.legible

        Repeater {
            model: Hg.art(root.condition, root.night)

            delegate: Text {
                required property string modelData

                text: modelData
                textFormat: Text.PlainText
                font.family: root.theme.monoFont
                font.pixelSize: root.fontPx
                color: root.artColour
                height: Math.round(root.fontPx * 1.2)
                verticalAlignment: Text.AlignVCenter
            }
        }
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.legible
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
