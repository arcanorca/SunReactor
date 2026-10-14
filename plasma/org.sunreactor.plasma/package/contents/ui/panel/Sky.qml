/*
 * Panel "sky" mode: the weather the policy reacts to, drawn in the theme's own
 * weather style. The theme's icon is loaded from the registry (components/
 * weathericons, Modules.weatherIcon) and sits in the panel's own colours. When
 * the user picked a weather icon style in settings (line, solid or pixel), the
 * generic glyph or sprite is drawn instead. The pixel sprite is drawn at a whole
 * number of device pixels per art pixel, so it stays crisp at fractional scales.
 * PixelSkySprite decodes the sprite at its art size; a QML Image would render the
 * SVG at the window ratio and resample it.
 *
 * A paused daemon shows the pause symbol instead.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../components/weathericons"
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

    /*! The shared sprite (generic pixel style) and the family sprite it may be swapped for. */
    readonly property url sharedSprite: Icons.conditionArt(condition, night)
    readonly property string spriteName: sharedSprite.toString().replace(/^.*\//, "").replace(/\.svg$/, "")
    readonly property url familySprite: view.theme.isPixel
        ? Qt.resolvedUrl("../skins/pixel/sky/" + spriteName + ".png") : ""
    /*! Device px per art px: the largest whole multiple of the 16 grid that fits. */
    readonly property int multiple: Math.max(1, Math.floor(view.thickness * view.dpr / 16 + 0.001))
    property bool familyMissing: false

    onFamilySpriteChanged: familyMissing = false

    /*! The generic glyph's side: the panel's graphic box, in whole logical px. */
    readonly property int glyphSize: Math.round(view.graphicSize)
    /*! The user picked a weather icon style: the generic glyph or sprite, not the theme's icon. */
    readonly property bool styleChosen: ["line", "solid", "pixel"].indexOf(view.theme.overrides.weatherIconStyle) >= 0
    readonly property bool themed: !paused && view.theme.available && !styleChosen
    readonly property bool useSprite: !paused && !themed && view.theme.glyphStyle === "pixel"
    readonly property bool useGlyph: !paused && !themed && !useSprite

    implicitWidth: paused ? icon.implicitWidth
        : (themed ? themedIcon.implicitWidth : (useSprite ? sprite.implicitWidth : glyph.implicitWidth))
    implicitHeight: paused ? icon.implicitHeight
        : (themed ? themedIcon.implicitHeight : (useSprite ? sprite.implicitHeight : glyph.implicitHeight))

    ThemedWeatherIcon {
        id: themedIcon

        visible: root.themed
        theme: root.view.theme
        condition: root.condition
        night: root.night
        size: root.view.graphicSize
        surface: root.view.panelBackground
        ink: root.view.panelText
    }

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
