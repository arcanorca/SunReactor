/*
 * The popup's light-blue frame (Modules.OVERLAY): four bands of
 * C64Style.frameWidth art px at the edge of the popup, over the blue screen fill
 * and under the content. Art px only, so the frame is exact at every ratio.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

PixelSurface {
    id: root

    required property Theme theme

    anchors.fill: parent
    grid: root.theme.grid
    fillsWidth: true
    /*! The popup's footer (refresh and theme buttons) is not screen: the frame closes above it. */
    readonly property var popupRoot: parent && parent.parent ? parent.parent.parent : null
    readonly property real screenBottom: popupRoot && popupRoot.footer && popupRoot.footer.visible
        ? popupRoot.footer.y : height
    artHeight: Math.floor(screenBottom / grid.unit)
    regionName: "c64-frame"

    readonly property int t: 3

    Rectangle {
        x: 0
        y: 0
        width: root.artWidth
        height: root.t
        color: root.theme.borderActive
        antialiasing: false
    }
    Rectangle {
        x: 0
        y: root.artHeight - root.t
        width: root.artWidth
        height: root.t
        color: root.theme.borderActive
        antialiasing: false
    }
    Rectangle {
        x: 0
        y: 0
        width: root.t
        height: root.artHeight
        color: root.theme.borderActive
        antialiasing: false
    }
    Rectangle {
        x: root.artWidth - root.t
        y: 0
        width: root.t
        height: root.artHeight
        color: root.theme.borderActive
        antialiasing: false
    }
}
