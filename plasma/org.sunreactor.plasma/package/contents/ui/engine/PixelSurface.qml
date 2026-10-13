/*
 * Renders its children at art resolution and shows them at an integer number
 * of device pixels per art pixel (DESIGN.md section 4, pixel rule 2).
 *
 * Children are laid out in ART PIXELS: x: 3 means three art pixels. The inner
 * item is `artWidth x artHeight` logical units, rendered into a layer texture
 * of exactly that many texels (1 unit = 1 texel), then scaled by grid.unit
 * (= k / dpr) with nearest filtering, so every art pixel becomes a k x k block
 * of identical device pixels at any device pixel ratio.
 *
 * What may go inside (verified by tools/pixel_proof.sh at DPR 1.0/1.35/2.0):
 *   - Rectangle / Item geometry at whole art-pixel coordinates, no radius,
 *     no antialiasing, no opacity < 1 (opacity blends: off-palette);
 *   - BitmapText and Dither with inSurface: true (CPU-rasterised 1:1);
 *   - Image with smooth: false at its natural size (PNG, no SVG).
 * What must not: QML Text (rasterised at the window DPR - blurs), Canvas,
 * Shapes with antialiasing, gradients, MultiEffect.
 *
 * Use `fillsWidth: true` to take whatever width the layout gives and fit as
 * many whole art pixels as possible (artWidth follows), else set artWidth.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: root

    required property PixelGrid grid
    property int artWidth: fillsWidth ? grid.cells(width) : 0
    property int artHeight: 0
    property bool fillsWidth: false
    /*! Region name reported to the screenshot harness (SUNREACTOR_HARNESS). */
    property string regionName: ""
    default property alias content: art.data
    readonly property real unit: grid.unit
    readonly property Item canvas: art

    implicitWidth: fillsWidth ? 0 : artWidth * unit
    implicitHeight: artHeight * unit

    Item {
        id: art

        width: root.artWidth
        height: root.artHeight
        transformOrigin: Item.TopLeft
        scale: root.unit
        layer.enabled: root.artWidth > 0 && root.artHeight > 0
        layer.textureSize: Qt.size(root.artWidth, root.artHeight)
        layer.smooth: false
        layer.mipmap: false
    }

    // The harness checks pixel areas for off-palette colours; it needs to know
    // where they ended up after layout, in device pixels.
    function reportRegion() {
        if (!root.reporting || !root.visible || root.artWidth <= 0 || root.artHeight <= 0) {
            return;
        }
        const r = root.grid.deviceRect(art);
        console.info("SUNREACTOR_PIXEL_REGION " + JSON.stringify({
            name: root.regionName || "surface",
            x: r.x, y: r.y, w: r.w, h: r.h,
            k: root.grid.deviceScale, dpr: root.grid.devicePixelRatio,
        }));
    }

    Timer {
        id: reportTimer
        interval: 700
        onTriggered: root.reportRegion()
    }

    readonly property bool reporting: grid !== null && grid.harness

    onXChanged: if (reporting) reportTimer.restart()
    onYChanged: if (reporting) reportTimer.restart()
    onArtWidthChanged: if (reporting) reportTimer.restart()
    onArtHeightChanged: if (reporting) reportTimer.restart()
    Component.onCompleted: if (reporting) reportTimer.restart()
}
