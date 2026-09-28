/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami

PlasmaComponents3.Slider {
    id: control

    property var tokens: null
    property color customAccent: control.tokens ? control.tokens.accentColor : Kirigami.Theme.highlightColor

    implicitHeight: 24

    background: Item {
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        width: control.availableWidth
        height: (control.tokens && (control.tokens.isAmiga || control.tokens.isUnix)) ? 8
            : ((control.tokens && (control.tokens.isHandheld || control.tokens.isC64)) ? 6 : 4)

        // 1. Amiga & Unix Recessed 3D Bevel Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && (control.tokens.isAmiga || control.tokens.isUnix)
            color: control.tokens && control.tokens.isUnix ? "#262E34" : "#223344"
            border.width: 0

            // Dark top & left
            Rectangle {
                anchors.left: parent.left; anchors.top: parent.top; anchors.right: parent.right; height: 1
                color: control.tokens ? control.tokens.bevelDark : "#000000"
            }
            Rectangle {
                anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 1
                color: control.tokens ? control.tokens.bevelDark : "#000000"
            }
            // Light bottom & right
            Rectangle {
                anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.right: parent.right; height: 1
                color: control.tokens ? control.tokens.bevelLight : "#99BBDD"
            }
            Rectangle {
                anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 1
                color: control.tokens ? control.tokens.bevelLight : "#99BBDD"
            }

            // Progress fill
            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: 1
                width: Math.max(0, (parent.width - 2) * control.visualPosition)
                color: control.tokens ? control.tokens.accentColor : "#FF8800"
            }
        }

        // 2. Handheld 4-Shade Pixel Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isHandheld
            color: "#8BAC0F"
            border.width: 1
            border.color: "#0F380F"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: 1
                width: Math.max(0, (parent.width - 2) * control.visualPosition)
                color: "#0F380F"
            }
        }

        // 3. VFD Cyan Phosphor Glow Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isVfd
            radius: 2
            color: "#051A13"
            border.width: 1
            border.color: Qt.rgba(0, 0.94, 0.66, 0.3)

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: 2
                width: parent.width * control.visualPosition
                color: "#00F0A8"
                opacity: 0.85
            }
        }

        // 4. Nixie Smoked Glass Neon Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isNixie
            radius: 2
            color: "#0B0A09"
            border.width: 1
            border.color: "#3D2413"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: 2
                width: parent.width * control.visualPosition
                color: "#FF7A18"
                opacity: 0.75
            }
        }

        // 5. Braun Concave Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isBraun
            radius: 2
            color: "#18181A"
            border.width: 1
            border.color: "#2C2C2E"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: 2
                width: parent.width * control.visualPosition
                color: "#F0B01E"
            }
        }

        // 6. ThinkPad Raven Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isThinkPad
            radius: 2
            color: "#141416"
            border.width: 1
            border.color: "#2E2E32"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: 2
                width: parent.width * control.visualPosition
                color: control.tokens ? control.tokens.accentColor : "#DA291C"
            }
        }

        // 7. Commodore 64 Pixel Track
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isC64
            color: "#40318D"
            border.width: 1
            border.color: "#8870E6"

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.margins: 1
                width: Math.max(0, (parent.width - 2) * control.visualPosition)
                color: "#A0A0FF"
            }
        }

        // 8. Nothing / Standard HIG Groove
        Rectangle {
            anchors.fill: parent
            visible: !control.tokens || (!control.tokens.isAmiga && !control.tokens.isUnix
                                         && !control.tokens.isHandheld && !control.tokens.isVfd
                                         && !control.tokens.isNixie && !control.tokens.isBraun
                                         && !control.tokens.isThinkPad && !control.tokens.isC64)
            radius: (control.tokens && control.tokens.isNothing) ? 2 : height / 2
            color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                           Kirigami.Theme.textColor.b, 0.15)

            Rectangle {
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                radius: parent.radius
                width: parent.width * control.visualPosition
                color: control.customAccent
            }
        }
    }

    handle: Item {
        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + (control.availableHeight - height) / 2
        width: (control.tokens && (control.tokens.isAmiga || control.tokens.isUnix)) ? 14
            : ((control.tokens && control.tokens.isHandheld) ? 12
            : ((control.tokens && control.tokens.isC64) ? 10 : 16))
        height: (control.tokens && (control.tokens.isAmiga || control.tokens.isUnix)) ? 20
            : ((control.tokens && (control.tokens.isHandheld || control.tokens.isC64)) ? 18 : 16)

        // 1. Amiga & Unix Raised 3D Bevel Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && (control.tokens.isAmiga || control.tokens.isUnix)
            color: control.tokens && control.tokens.isUnix ? "#4E5D6C" : "#5588BB"

            // Light top & left
            Rectangle {
                anchors.left: parent.left; anchors.top: parent.top; anchors.right: parent.right; height: 2
                color: control.tokens ? control.tokens.bevelLight : "#FFFFFF"
            }
            Rectangle {
                anchors.left: parent.left; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 2
                color: control.tokens ? control.tokens.bevelLight : "#FFFFFF"
            }
            // Dark bottom & right
            Rectangle {
                anchors.left: parent.left; anchors.bottom: parent.bottom; anchors.right: parent.right; height: 2
                color: control.tokens ? control.tokens.bevelDark : "#000000"
            }
            Rectangle {
                anchors.right: parent.right; anchors.top: parent.top; anchors.bottom: parent.bottom; width: 2
                color: control.tokens ? control.tokens.bevelDark : "#000000"
            }

            // Inner accent gadget mark
            Rectangle {
                width: 4
                height: 8
                anchors.centerIn: parent
                color: control.tokens ? control.tokens.accentColor : "#FF8800"
            }
        }

        // 2. Handheld Burgundy DMG Button Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isHandheld
            color: control.tokens ? control.tokens.burgundyAccent : "#8B1D42"
            border.width: 1
            border.color: "#0F380F"

            Rectangle {
                width: 4
                height: 4
                anchors.centerIn: parent
                color: "#9BBC0F"
            }
        }

        // 3. VFD Cyan Glow Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isVfd
            radius: 3
            color: "#00F0A8"
            border.width: 1
            border.color: "#FFFFFF"

            Rectangle {
                anchors.fill: parent
                anchors.margins: -2
                radius: 5
                color: "transparent"
                border.width: 1
                border.color: "#00F0A8"
                opacity: 0.5
            }
        }

        // 4. Nixie Neon Bead Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isNixie
            radius: 8
            color: "#FFB765"
            border.width: 2
            border.color: "#FF7A18"

            Rectangle {
                anchors.fill: parent
                anchors.margins: -2
                radius: 10
                color: "transparent"
                border.width: 1
                border.color: "#FF7A18"
                opacity: 0.6
            }
        }

        // 5. Braun Concave Circular Pill Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isBraun
            radius: 8
            color: "#303032"
            border.width: 1
            border.color: "#18181A"

            Rectangle {
                width: 6
                height: 6
                radius: 3
                anchors.centerIn: parent
                color: "#FF5500" // Iconic power orange
            }
        }

        // 6. ThinkPad TrackPoint Red Cap Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isThinkPad
            radius: 8
            color: control.tokens ? control.tokens.liveIndicatorColor : "#DA291C"
            border.width: 1
            border.color: "#8B0000"

            Rectangle {
                width: 4
                height: 4
                radius: 2
                anchors.centerIn: parent
                color: "#FF4D4D"
            }
        }

        // 7. Commodore 64 Pixel Block Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isC64
            color: "#8870E6"
            border.width: 1
            border.color: "#A0A0FF"

            Rectangle {
                width: 4
                height: 6
                anchors.centerIn: parent
                color: "#A0A0FF"
            }
        }

        // 8. Nothing OS Dot Handle
        Rectangle {
            anchors.fill: parent
            visible: control.tokens && control.tokens.isNothing
            radius: 8
            color: "#FFFFFF"
            border.width: 1
            border.color: Qt.rgba(0, 0, 0, 0.25)

            Rectangle {
                width: 4
                height: 4
                radius: 2
                anchors.centerIn: parent
                color: control.tokens ? control.tokens.liveIndicatorColor : "#E50914"
            }
        }

        // 9. Standard HIG Handle
        Rectangle {
            anchors.fill: parent
            visible: !control.tokens || (!control.tokens.isAmiga && !control.tokens.isUnix
                                         && !control.tokens.isHandheld && !control.tokens.isVfd
                                         && !control.tokens.isNixie && !control.tokens.isBraun
                                         && !control.tokens.isThinkPad && !control.tokens.isC64
                                         && !control.tokens.isNothing)
            radius: 8
            color: control.pressed ? control.customAccent : "#FFFFFF"
            border.width: 1
            border.color: Qt.rgba(0, 0, 0, 0.25)
        }
    }
}
