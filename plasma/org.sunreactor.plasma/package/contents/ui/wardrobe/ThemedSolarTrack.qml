/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "nothing"
import "handheld"
import "amiga"
import "vfd"
import "nixie"
import "../Icons.js" as Icons

Item {
    id: solarTrack

    property var tokens: null
    property real dayProgress: 0.0 // 0.0 at sunrise, 1.0 at sunset
    property bool daylight: true
    property color accentColor: solarTrack.tokens ? solarTrack.tokens.accentColor : Kirigami.Theme.highlightColor

    implicitHeight: 18

    // 1. Nothing OS Dot-Matrix Track
    NothingDotMatrixTrack {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isNothing
        progress: solarTrack.dayProgress
        daylight: solarTrack.daylight
        activeColor: solarTrack.tokens ? solarTrack.tokens.textColor : "#FFFFFF"
        dotColor: solarTrack.tokens ? solarTrack.tokens.liveIndicatorColor : "#E50914"
    }

    // 2. Handheld 4-Shade Pixel Track
    HandheldPixelTrack {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isHandheld
        progress: solarTrack.dayProgress
        daylight: solarTrack.daylight
        burgundy: solarTrack.tokens ? solarTrack.tokens.burgundyAccent : "#8B1D42"
    }

    // 3. VFD Hi-Fi Fluorescent Segment Bar
    VfdSegmentGauge {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isVfd
        value: solarTrack.dayProgress
        phosphorCyan: solarTrack.tokens ? solarTrack.tokens.accentColor : "#00F0A8"
        phosphorAmber: solarTrack.tokens ? solarTrack.tokens.secondaryAccentColor : "#FFB000"
    }

    // 4. Nixie Cold-Cathode Tube Glow
    NixieCathodeGlow {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isNixie
        progress: solarTrack.dayProgress
        neonGlow: solarTrack.tokens ? solarTrack.tokens.accentColor : "#FF7A18"
        neonCore: solarTrack.tokens ? solarTrack.tokens.secondaryAccentColor : "#FFB765"
    }

    // 5. Amiga Workbench 3D Bevelled Track with Boing Ball
    Item {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isAmiga

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: 6
            color: "#223344"
            border.width: 1
            border.color: "#000000"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                color: "#FF8800"
            }
        }

        AmigaBoingBall {
            size: 18
            x: Math.max(0, Math.min(parent.width - size, (parent.width - size) * solarTrack.dayProgress))
            anchors.verticalCenter: parent.verticalCenter
            running: true
        }
    }

    // 6. Braun ET66 Precision Silkscreen Track
    Item {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isBraun

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: 3
            radius: 1.5
            color: "#2C2C2E"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                radius: parent.radius
                color: "#F0B01E" // Braun yellow
            }
        }

        // Circular concave indicator button
        Rectangle {
            width: 10
            height: 10
            radius: 5
            x: Math.max(0, Math.min(parent.width - width, (parent.width - width) * solarTrack.dayProgress))
            anchors.verticalCenter: parent.verticalCenter
            color: "#FF5500" // Braun iconic power orange
            border.width: 1
            border.color: "#333333"
        }
    }

    // 7. Unix Workstation 3D Bevelled Track with Jewel Lamp
    Item {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isUnix

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: 6
            color: "#262E34"
            border.width: 1
            border.color: "#181D26"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                color: solarTrack.tokens ? solarTrack.tokens.accentColor : "#4580A0"
            }
        }

        Rectangle {
            width: 8
            height: 14
            radius: 1
            x: Math.max(0, Math.min(parent.width - width, (parent.width - width) * solarTrack.dayProgress))
            anchors.verticalCenter: parent.verticalCenter
            color: solarTrack.tokens ? solarTrack.tokens.liveIndicatorColor : "#00FF7F"
            border.width: 1
            border.color: "#FFFFFF"
        }
    }

    // 8. ThinkPad Raven Track with TrackPoint Pip
    Item {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isThinkPad

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: 4
            radius: 2
            color: "#202024"
            border.width: 1
            border.color: "#2E2E32"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                radius: parent.radius
                color: "#404044"
            }
        }

        Rectangle {
            width: 8
            height: 8
            radius: 4
            x: Math.max(0, Math.min(parent.width - width, (parent.width - width) * solarTrack.dayProgress))
            anchors.verticalCenter: parent.verticalCenter
            color: solarTrack.tokens ? solarTrack.tokens.liveIndicatorColor : "#DA291C"
            border.width: 1
            border.color: "#8B0000"
        }
    }

    // 9. Commodore 64 Pixel Track
    Item {
        anchors.fill: parent
        visible: solarTrack.tokens && solarTrack.tokens.isC64

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: 6
            color: "#40318D"
            border.width: 1
            border.color: "#8870E6"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                color: "#A0A0FF"
            }
        }

        Rectangle {
            width: 6
            height: 10
            x: Math.max(0, Math.min(parent.width - width, (parent.width - width) * solarTrack.dayProgress))
            anchors.verticalCenter: parent.verticalCenter
            color: "#8870E6"
            border.width: 1
            border.color: "#FFFFFF"
        }
    }

    // 10. Standard / Modern GNOME HIG Track
    Item {
        anchors.fill: parent
        visible: !solarTrack.tokens || (!solarTrack.tokens.isNothing && !solarTrack.tokens.isHandheld
                                       && !solarTrack.tokens.isVfd && !solarTrack.tokens.isNixie
                                       && !solarTrack.tokens.isAmiga && !solarTrack.tokens.isBraun
                                       && !solarTrack.tokens.isUnix && !solarTrack.tokens.isThinkPad
                                       && !solarTrack.tokens.isC64)

        Rectangle {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            height: Math.max(2, Kirigami.Units.smallSpacing / 2)
            radius: height / 2
            color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                           Kirigami.Theme.textColor.b, 0.15)

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                width: parent.width * solarTrack.dayProgress
                radius: parent.radius
                color: Qt.rgba(solarTrack.accentColor.r, solarTrack.accentColor.g,
                               solarTrack.accentColor.b, solarTrack.daylight ? 0.75 : 0.35)
            }
        }

        Kirigami.Icon {
            width: 16
            height: 16
            x: (parent.width - width) * solarTrack.dayProgress
            anchors.verticalCenter: parent.verticalCenter
            roundToIconSize: false
            source: Icons.skyArt(!solarTrack.daylight)

            Behavior on x {
                NumberAnimation {
                    duration: Kirigami.Units.longDuration
                    easing.type: Easing.InOutQuad
                }
            }
        }
    }
}
