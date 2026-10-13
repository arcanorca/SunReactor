/*
 * Panel "sky" mode: the weather the policy reacts to. In the theme's weather
 * icon style it is a vector glyph (WeatherGlyph), in the panel's own colours
 * where the theme allows. With the "pixel" style it is the 16 px sprite at a
 * whole number of device pixels per art pixel (16, 32, 48 ... device px), so it
 * stays crisp at fractional scales. PixelSkySprite decodes the sprite at its art
 * size; a QML Image would render the SVG at the window ratio and resample it.
 *
 * Pixel themes look for a family sprite first - skins/pixel/sky/<name>.png
 * (16x16, same names as icons/weather/*.svg) - and fall back to the shared
 * sprite when there is none. A paused daemon shows the pause symbol instead.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Icons.js" as Icons

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

    /*! The shared sprite (pixel style) and the family sprite it may be swapped for. */
    readonly property url sharedSprite: Icons.conditionArt(condition, night)
    readonly property string spriteName: sharedSprite.toString().replace(/^.*\//, "").replace(/\.svg$/, "")
    readonly property url familySprite: view.theme.isPixel
        ? Qt.resolvedUrl("../skins/pixel/sky/" + spriteName + ".png") : ""
    /*! Device px per art px: the largest whole multiple of the 16 grid that fits. */
    readonly property int multiple: Math.max(1, Math.floor(view.thickness * view.dpr / 16 + 0.001))
    property bool familyMissing: false

    onFamilySpriteChanged: familyMissing = false

    /*! The glyph's side: the panel's graphic box, in whole logical px. */
    readonly property int glyphSize: Math.round(view.graphicSize)
    readonly property bool useSprite: view.theme.glyphStyle === "pixel" && !paused
    readonly property bool useGlyph: !paused && !useSprite

    implicitWidth: paused ? icon.implicitWidth : (useSprite ? sprite.implicitWidth : glyph.implicitWidth)
    implicitHeight: paused ? icon.implicitHeight : (useSprite ? sprite.implicitHeight : glyph.implicitHeight)

    PixelSkySprite {
        id: sprite

        visible: root.useSprite
        // Only the sprite path loads anything; the glyph path must not fetch PNGs.
        source: !root.useSprite ? ""
            : (root.familySprite.toString().length > 0 && !root.familyMissing ? root.familySprite
                                                                                 : root.sharedSprite)
        artSize: 16
        artScale: root.multiple / root.view.dpr
        onStatusChanged: {
            if (status === PixelSkySprite.Error && source === root.familySprite) {
                // Deferred: a synchronous error would re-enter this source binding.
                Qt.callLater(() => { root.familyMissing = true; });
            }
        }
    }

    WeatherGlyph {
        id: glyph

        visible: root.useGlyph
        condition: root.condition
        night: root.night
        glyphStyle: root.view.theme.glyphStyle
        // The theme's own colours, lifted to 3:1 against the panel where needed.
        palette: root.view.theme.glyphColours(root.view.panelBackground, root.view.theme.fg)
        size: root.glyphSize
    }

    Kirigami.Icon {
        id: icon

        visible: root.paused
        implicitWidth: Kirigami.Units.iconSizes.roundedIconSize(root.view.graphicSize)
        implicitHeight: implicitWidth
        source: root.view.skySource
        active: root.view.hovered
    }
}
