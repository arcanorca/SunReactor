/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: card

    property var tokens: null
    property real contentImplicitWidth: 0
    property real contentImplicitHeight: 0
    default property alias content: innerContainer.data

    readonly property int bevelOffset: (card.tokens && card.tokens.isBevel3D) ? 4 : 0

    implicitWidth: (contentImplicitWidth > 0 ? contentImplicitWidth : innerContainer.childrenRect.width) + bevelOffset
    implicitHeight: (contentImplicitHeight > 0 ? contentImplicitHeight : innerContainer.childrenRect.height) + bevelOffset

    // Base background rectangle
    Rectangle {
        id: bg
        anchors.fill: parent
        radius: card.tokens ? card.tokens.cardRadius : 10
        color: card.tokens ? card.tokens.cardBgColor : Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.04)
        border.width: (card.tokens && card.tokens.isBevel3D) ? 0 : (card.tokens ? card.tokens.cardBorderWidth : 1)
        border.color: card.tokens ? card.tokens.cardBorderColor : Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g, Kirigami.Theme.textColor.b, 0.08)
        clip: true

        // 3D Bevel for Amiga Workbench and Unix Workstation
        // Top and Left light highlight
        Rectangle {
            visible: card.tokens && card.tokens.isBevel3D
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.right: parent.right
            height: 2
            color: card.tokens ? card.tokens.bevelLight : "#FFFFFF"
        }
        Rectangle {
            visible: card.tokens && card.tokens.isBevel3D
            anchors.left: parent.left
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            color: card.tokens ? card.tokens.bevelLight : "#FFFFFF"
        }
        // Bottom and Right dark shadow
        Rectangle {
            visible: card.tokens && card.tokens.isBevel3D
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: 2
            color: card.tokens ? card.tokens.bevelDark : "#000000"
        }
        Rectangle {
            visible: card.tokens && card.tokens.isBevel3D
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            width: 2
            color: card.tokens ? card.tokens.bevelDark : "#000000"
        }

        // Texture overlay (Reticle, mesh, DMG pattern, filaments)
        Image {
            id: textureImg
            anchors.fill: parent
            visible: card.tokens && card.tokens.backgroundTexture && card.tokens.backgroundTexture.length > 0 && card.tokens.textureOpacity > 0
            source: (card.tokens && card.tokens.backgroundTexture) ? card.tokens.backgroundTexture : ""
            opacity: card.tokens ? card.tokens.textureOpacity : 0.0
            fillMode: Image.Tile
            asynchronous: true
            cache: true
        }

        // Nothing OS hairline red live dot indicator in top corner
        Rectangle {
            visible: card.tokens && card.tokens.isNothing
            width: 5
            height: 5
            radius: 2.5
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 6
            color: card.tokens ? card.tokens.liveIndicatorColor : "#E50914"

            SequentialAnimation on opacity {
                running: card.tokens && card.tokens.isNothing && card.visible
                loops: Animation.Infinite
                NumberAnimation { from: 1.0; to: 0.35; duration: 1200; easing.type: Easing.InOutQuad }
                NumberAnimation { from: 0.35; to: 1.0; duration: 1200; easing.type: Easing.InOutQuad }
            }
        }

        // Handheld DMG speaker grill dots ornament in bottom right
        Row {
            visible: card.tokens && card.tokens.isHandheld
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.margins: 4
            spacing: 2
            Repeater {
                model: 3
                Rectangle {
                    required property int index
                    width: 2
                    height: 8
                    radius: 1
                    color: card.tokens ? card.tokens.cardBorderColor : "#0F380F"
                    opacity: 0.4
                    rotation: 25
                }
            }
        }

        // VFD phosphor glow rim effect
        Rectangle {
            visible: card.tokens && card.tokens.isVfd
            anchors.fill: parent
            radius: card.tokens ? card.tokens.cardRadius : 4
            color: "transparent"
            border.width: 1
            border.color: card.tokens ? card.tokens.accentColor : "#00F0A8"
            opacity: 0.35
        }

        // Inner container for content
        Item {
            id: innerContainer
            anchors.fill: parent
            anchors.margins: (card.tokens && card.tokens.isBevel3D) ? 2 : 0
        }
    }
}
