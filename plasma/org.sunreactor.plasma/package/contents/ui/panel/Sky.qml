/*
 * Panel "sky" mode: the weather the policy reacts to, drawn in the style the user
 * chose (components/weathericons/ThemedWeatherIcon): the theme's own icon by
 * default, or the modern, outline, filled or pixel icon in every theme. The panel's
 * own colours are the surface. A paused daemon shows the pause symbol instead; on
 * the Game Boy DMG (handheld) the pause symbol is drawn from its pixel atlas in the
 * four shades of its ramp, on a tile of the lightest shade, like its LCD.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../components/weathericons"
import "../engine"
import "../skins/pixel/Sprites.js" as Sprites

Item {
    id: root

    required property PanelView view

    readonly property SunReactorClient client: view.client
    /*! Paused: the sky is not driving anything, so the pause symbol stands in. */
    readonly property bool paused: client.isConnected && client.isSuspended
    /*! No reading yet means the sun or moon of the moment; not connected, unknown. */
    readonly property string condition: !client.isConnected ? ""
                                        : (client.hasWeatherReading ? client.weatherCondition : "clear")
    readonly property bool night: client.hasWeatherReading ? client.weatherIsNight : !client.isDaylight

    /*! The Game Boy DMG: the pause symbol in its four shades. */
    readonly property bool mono: Sprites.isMonoRamp(view.theme)
    readonly property var shades: view.theme.shades

    implicitWidth: paused ? (mono ? monoPause.implicitWidth : icon.implicitWidth) : themedIcon.implicitWidth
    implicitHeight: paused ? (mono ? monoPause.implicitHeight : icon.implicitHeight) : themedIcon.implicitHeight

    ThemedWeatherIcon {
        id: themedIcon

        visible: !root.paused
        theme: root.view.theme
        condition: root.condition
        night: root.night
        size: root.view.graphicSize
        surface: root.view.panelBackground
        ink: root.view.panelText
    }

    /*! The DMG while paused: the pause symbol on its LCD tile, in the four shades. */
    Item {
        id: monoPause

        readonly property real dpr: root.view.dpr
        /*! Device px per art px of the 16 px grid, as the weather sprite; the 8 px symbol takes two of them. */
        readonly property int artPx: Math.max(1, Math.floor(width * dpr / 16 + 0.001))

        visible: root.paused && root.mono
        width: root.view.graphicSize
        height: width
        implicitWidth: width
        implicitHeight: height

        Rectangle {
            anchors.fill: parent
            color: root.shades[root.shades.length - 1]
        }
        PixelSprite {
            x: Math.round((monoPause.width * monoPause.dpr - 16 * monoPause.artPx) / 2) / monoPause.dpr
            y: Math.round((monoPause.height * monoPause.dpr - 16 * monoPause.artPx) / 2) / monoPause.dpr
            source: Sprites.symbolAtlas()
            frame: Sprites.symbol("pause")
            frameWidth: 8
            frameHeight: 8
            colors: { const c = Sprites.monoColors("pause", root.shades); c[8] = "transparent"; return c; }
            artScale: 2 * monoPause.artPx / monoPause.dpr
        }
    }

    Kirigami.Icon {
        id: icon

        visible: root.paused && !root.mono
        implicitWidth: Kirigami.Units.iconSizes.roundedIconSize(root.view.graphicSize)
        implicitHeight: implicitWidth
        source: root.view.skySource
        active: root.view.hovered
    }
}
