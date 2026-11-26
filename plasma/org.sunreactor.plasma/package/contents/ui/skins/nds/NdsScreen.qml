/*
 * Nintendo DS hardware screen shell:
 *  - Top screen ("sky"): pearl DS Lite bezel with stereo speaker grilles
 *    (assets/ds_top_bezel.png) and cylindrical barrel hinge (assets/ds_hinge.png)
 *    with a live Power/Wi-Fi LED reflecting daemon mode.
 *  - Bottom screen ("touch"): recessed touch bezel (assets/ds_bottom_bezel.png)
 *    over a PictoChat graph-paper grid (assets/ds_picto_grid.png).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"

Item {
    id: root

    required property Theme theme
    required property NdsStyle style
    /*! "sky" (top screen) or "touch" (bottom screen). */
    property string wallpaper: "touch"
    /*! Height of the glass, logical px. */
    property real glassHeight: 100
    /*! Draw the hinge strip under the bezel (the top screen). */
    property bool hinge: false
    /*! Daemon mode for the hinge status LEDs (SunReactorClient.Mode). */
    property int daemonMode: SunReactorClient.Automatic

    readonly property real glassWidth: Math.max(1, root.width - 2 * root.style.bezel)

    default property alias content: stage.data

    implicitHeight: bezelBox.height + (root.hinge ? root.style.hingeHeight : 0)

    readonly property color ledColor: {
        switch (daemonMode) {
        case SunReactorClient.Offline: return root.style.danger;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual: return root.style.warning;
        default: return "#38d458";
        }
    }

    BorderImage {
        id: bezelBox

        width: root.width
        height: root.glassHeight + 2 * root.style.bezel
        source: Qt.resolvedUrl(root.wallpaper === "sky" ? "assets/ds_top_bezel.png" : "assets/ds_bottom_bezel.png")
        border { left: 8; right: 8; top: 8; bottom: 8 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: false
    }

    Item {
        id: glass

        x: root.style.bezel
        y: root.style.bezel
        width: root.glassWidth
        height: root.glassHeight
        clip: true

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: root.wallpaper === "sky" ? root.style.skyTop : root.style.touchTop
                }
                GradientStop {
                    position: 0.5
                    color: root.wallpaper === "sky" ? root.style.skyMid : root.style.touchMid
                }
                GradientStop {
                    position: 1
                    color: root.wallpaper === "sky" ? root.style.skyBottom : root.style.touchBottom
                }
            }
        }

        Image {
            anchors.fill: parent
            source: Qt.resolvedUrl(root.wallpaper === "sky" ? "assets/ds_sky_grid.png" : "assets/ds_picto_grid.png")
            fillMode: Image.Tile
            smooth: false
            opacity: root.wallpaper === "sky" ? 0.55 : 0.85
        }

        Item {
            id: stage

            anchors.fill: parent
        }
    }

    // The cylindrical barrel hinge with camera lens & crisp pixel LED indicators
    Item {
        visible: root.hinge
        x: 0
        y: bezelBox.height
        width: root.width
        height: root.style.hingeHeight

        BorderImage {
            anchors.fill: parent
            source: Qt.resolvedUrl("assets/ds_hinge.png")
            border { left: 36; right: 36; top: 0; bottom: 0 }
            horizontalTileMode: BorderImage.Stretch
            verticalTileMode: BorderImage.Stretch
            smooth: false
        }

        // Centered DSi camera / mic module (unstretched)
        Image {
            anchors.centerIn: parent
            width: Math.round(parent.height * 2.2)
            height: Math.round(parent.height * 0.78)
            source: Qt.resolvedUrl("assets/ds_hinge_camera.png")
            fillMode: Image.PreserveAspectFit
            smooth: false
        }

        // Crisp rectangular pixel LEDs centered inside the right hinge LED windows
        Rectangle {
            width: Math.max(3, Math.round(parent.height * 0.22))
            height: Math.max(4, Math.round(parent.height * 0.26))
            x: Math.round(root.width - 19.5 - width / 2)
            y: Math.round((parent.height - height) / 2)
            radius: 0
            color: root.ledColor
        }
        Rectangle {
            width: Math.max(3, Math.round(parent.height * 0.22))
            height: Math.max(4, Math.round(parent.height * 0.26))
            x: Math.round(root.width - 12.5 - width / 2)
            y: Math.round((parent.height - height) / 2)
            radius: 0
            color: root.daemonMode === SunReactorClient.Offline ? "#3a424e" : "#44c2ff"
        }
    }
}
