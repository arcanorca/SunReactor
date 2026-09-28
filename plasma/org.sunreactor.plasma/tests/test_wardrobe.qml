/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import "../package/contents/ui/wardrobe"
import "../package/contents/ui/wardrobe/amiga"

Item {
    id: testRoot
    width: 600
    height: 800

    QtObject {
        id: mockClient
        property string themeName: "nothing"
        property string themeAccent: "#FFFFFF"
        property string themeSecondaryAccent: "#E50914"
        property string themeBg: "#121216"
        property string themeFg: "#FFFFFF"
        property string themeTextMuted: "#888888"
        property bool isConnected: true
    }

    ThemeTokens {
        id: tokens
        client: mockClient
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 16

        WardrobeCard {
            id: testCard
            Layout.fillWidth: true
            tokens: tokens
            contentImplicitHeight: innerCol.implicitHeight + 16

            ColumnLayout {
                id: innerCol
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 8
                spacing: 8

                Text {
                    id: themeLabel
                    text: "Theme: " + tokens.displayName + " (" + tokens.archetypeKey + ")"
                    font.family: tokens.fontFamily
                    color: tokens.textColor
                }

                ThemedSolarTrack {
                    id: solarTrack
                    Layout.fillWidth: true
                    tokens: tokens
                    dayProgress: 0.65
                    daylight: true
                }

                ThemedSlider {
                    id: testSlider
                    Layout.fillWidth: true
                    tokens: tokens
                    value: 45
                }
            }
        }

        AmigaBoingBall {
            id: boingBall
            size: 24
        }

        GuruMeditationAlert {
            id: guruAlert
            Layout.fillWidth: true
            logicalId: "DP-2"
        }
    }

    Timer {
        id: testRunner
        interval: 50
        running: true
        repeat: true

        property var testCases: [
            { name: "nothing", archetype: "nothing", font: "Geist", digits: "Geist Mono", isLucent: true, is3D: false },
            { name: "handheld", archetype: "handheld", font: "Press Start 2P", digits: "Press Start 2P", isLucent: false, is3D: false },
            { name: "amiga", archetype: "amiga", font: "VT323", digits: "VT323", isLucent: false, is3D: true },
            { name: "braun", archetype: "braun", font: "IBM Plex Sans", digits: "IBM Plex Mono", isLucent: false, is3D: false },
            { name: "vfd_hifi", archetype: "vfd_hifi", font: "Orbitron", digits: "Orbitron", isLucent: false, is3D: false },
            { name: "nixie", archetype: "nixie", font: "IBM Plex Sans", digits: "VT323", isLucent: false, is3D: false },
            { name: "thinkpad", archetype: "thinkpad", font: "IBM Plex Sans", digits: "IBM Plex Mono", isLucent: false, is3D: false },
            { name: "unix_workstation", archetype: "unix_workstation", font: "IBM Plex Mono", digits: "IBM Plex Mono", isLucent: false, is3D: true },
            { name: "commodore64", archetype: "commodore64", font: "Sysfont", digits: "Sysfont", isLucent: false, is3D: false },
            { name: "modern", archetype: "modern", font: "", digits: "", isLucent: false, is3D: false },
            // Alias tests
            { name: "dmg", archetype: "handheld", font: "Press Start 2P", digits: "Press Start 2P", isLucent: false, is3D: false },
            { name: "workbench", archetype: "amiga", font: "VT323", digits: "VT323", isLucent: false, is3D: true },
            { name: "din", archetype: "braun", font: "IBM Plex Sans", digits: "IBM Plex Mono", isLucent: false, is3D: false },
            { name: "vfd", archetype: "vfd_hifi", font: "Orbitron", digits: "Orbitron", isLucent: false, is3D: false },
            { name: "tubes", archetype: "nixie", font: "IBM Plex Sans", digits: "VT323", isLucent: false, is3D: false },
            { name: "cde", archetype: "unix_workstation", font: "IBM Plex Mono", digits: "IBM Plex Mono", isLucent: false, is3D: true },
            { name: "c64", archetype: "commodore64", font: "Sysfont", digits: "Sysfont", isLucent: false, is3D: false },
            { name: "glyph", archetype: "nothing", font: "Geist", digits: "Geist Mono", isLucent: true, is3D: false }
        ]
        property int idx: 0

        onTriggered: {
            if (idx >= testCases.length) {
                console.log("SUCCESS: All " + testCases.length + " wardrobe archetypes and aliases verified!");
                Qt.quit();
                return;
            }

            var tc = testCases[idx];
            mockClient.themeName = tc.name;

            // Strict contract assertions
            if (tokens.archetypeKey !== tc.archetype) {
                console.error("FAIL: Theme '" + tc.name + "' expected archetype '" + tc.archetype + "', got '" + tokens.archetypeKey + "'");
                Qt.exit(1);
            }
            if (tokens.fontFamily !== tc.font) {
                console.error("FAIL: Theme '" + tc.name + "' expected fontFamily '" + tc.font + "', got '" + tokens.fontFamily + "'");
                Qt.exit(1);
            }
            if (tokens.digitFontFamily !== tc.digits) {
                console.error("FAIL: Theme '" + tc.name + "' expected digitFontFamily '" + tc.digits + "', got '" + tokens.digitFontFamily + "'");
                Qt.exit(1);
            }
            if (tokens.isBevel3D !== tc.is3D) {
                console.error("FAIL: Theme '" + tc.name + "' expected isBevel3D=" + tc.is3D + ", got " + tokens.isBevel3D);
                Qt.exit(1);
            }
            if (tokens.isLucent !== tc.isLucent) {
                console.error("FAIL: Theme '" + tc.name + "' expected isLucent=" + tc.isLucent + ", got " + tokens.isLucent);
                Qt.exit(1);
            }
            if (testCard.implicitHeight <= 0) {
                console.error("FAIL: WardrobeCard implicitHeight must be > 0, got " + testCard.implicitHeight);
                Qt.exit(1);
            }

            console.log("PASS [" + (idx + 1) + "/" + testCases.length + "]: '" + tc.name + "' -> archetype:" + tokens.archetypeKey + " (font:" + tokens.fontFamily + ")");
            idx++;
        }
    }
}
