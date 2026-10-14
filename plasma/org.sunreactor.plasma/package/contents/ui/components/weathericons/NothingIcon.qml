/*
 * Nothing (dot matrix): the twelve-by-twelve pictogram of lit dots, a whole device
 * pixel per dot pitch, centred on the device pixel grid. Each dot pitch must be at
 * least two device px, so below 24 device px across it draws the generic line
 * glyph instead: the dots would merge at panel size.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma
import "../../engine"
import "../../panel/PanelData.js" as PanelData

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
    /*! Device px per dot pitch: twelve pitches fill the icon's side. */
    readonly property int pitchPx: Math.floor(size * dpr / 12 + 0.001)
    readonly property bool dotted: pitchPx >= 2

    DotIcon {
        id: dots

        visible: root.dotted
        x: Math.round((root.width * root.dpr - 12 * root.pitchPx) / 2) / root.dpr
        y: Math.round((root.height * root.dpr - 12 * root.pitchPx) / 2) / root.dpr
        pitch: root.pitchPx / root.dpr
        condition: root.condition
        night: root.night
        litColor: root.ink
        accentColor: PanelData.ink(root.theme.accent, root.surface, root.ink, 3)
        dimColor: Qt.rgba(root.ink.r, root.ink.g, root.ink.b, 0.12)
    }

    WeatherGlyph {
        anchors.centerIn: parent
        visible: !root.dotted
        condition: root.condition
        night: root.night
        glyphStyle: "line"
        palette: root.theme.glyphColours(root.surface, root.ink)
        size: Math.round(root.size)
    }
}
