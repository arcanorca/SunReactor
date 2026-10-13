/*
 * Panel "sky" mode: the weather sprite at a whole number of device pixels per
 * art pixel (16-grid sprites: 16, 32, 48 ... device px), so it stays crisp at
 * fractional scales. PixelSkySprite decodes the sprite at its art size; a QML
 * Image would render the SVG at the window ratio and resample it.
 *
 * Pixel themes look for a family sprite first - skins/pixel/sky/<name>.png
 * (16x16, same names as icons/weather/*.svg) - and fall back to the shared
 * sprite when there is none. Theme icon names (paused) use Kirigami.Icon.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

Item {
    id: root

    required property PanelView view

    readonly property bool isSprite: /\.(svg|png)$/.test(view.skySource)
    readonly property string spriteName: isSprite ? view.skySource.replace(/^.*\//, "").replace(/\.(svg|png)$/, "") : ""
    readonly property url familySprite: view.theme.isPixel && isSprite
        ? Qt.resolvedUrl("../skins/pixel/sky/" + spriteName + ".png") : ""
    /*! Device px per art px: the largest whole multiple of the 16 grid that fits. */
    readonly property int multiple: Math.max(1, Math.floor(view.thickness * view.dpr / 16 + 0.001))
    property bool familyMissing: false

    onFamilySpriteChanged: familyMissing = false

    implicitWidth: isSprite ? sprite.implicitWidth : icon.implicitWidth
    implicitHeight: isSprite ? sprite.implicitHeight : icon.implicitHeight

    PixelSkySprite {
        id: sprite

        visible: root.isSprite
        source: root.familySprite.toString().length > 0 && !root.familyMissing ? root.familySprite
                                                                                : root.view.skySource
        artSize: 16
        artScale: root.multiple / root.view.dpr
        onStatusChanged: {
            if (status === PixelSkySprite.Error && source === root.familySprite) {
                root.familyMissing = true;
            }
        }
    }

    Kirigami.Icon {
        id: icon

        visible: !root.isSprite
        implicitWidth: Kirigami.Units.iconSizes.roundedIconSize(root.view.graphicSize)
        implicitHeight: implicitWidth
        source: root.isSprite ? "" : root.view.skySource
        active: root.view.hovered
    }
}
