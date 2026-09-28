/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

import QtQuick
import org.kde.kirigami as Kirigami

import "ThemeCatalog.js" as ThemeCatalog

Item {
    id: root

    property var client: null

    // Registered Font Assets
    FontLoader { id: geistSans; source: "../../fonts/Geist-Regular.ttf" }
    FontLoader { id: geistMono; source: "../../fonts/GeistMono-Regular.ttf" }
    FontLoader { id: pressStart; source: "../../fonts/PressStart2P.ttf" }
    FontLoader { id: vt323; source: "../../fonts/VT323.ttf" }
    FontLoader { id: sysfont; source: "../../fonts/Sysfont.ttf" }
    FontLoader { id: ibmPlexMono; source: "../../fonts/IBMPlexMono-Regular.ttf" }
    FontLoader { id: ibmPlexSans; source: "../../fonts/IBMPlexSans-Regular.ttf" }
    FontLoader { id: orbitron; source: "../../fonts/Orbitron-Variable.ttf" }

    readonly property string geistSansFamily: geistSans.status === FontLoader.Ready ? geistSans.name : "Geist"
    readonly property string geistMonoFamily: geistMono.status === FontLoader.Ready ? geistMono.name : "Geist Mono"
    readonly property string pressStartFamily: pressStart.status === FontLoader.Ready ? pressStart.name : "Press Start 2P"
    readonly property string vt323Family: vt323.status === FontLoader.Ready ? vt323.name : "VT323"
    readonly property string sysfontFamily: sysfont.status === FontLoader.Ready ? sysfont.name : "Sysfont"
    readonly property string ibmPlexMonoFamily: ibmPlexMono.status === FontLoader.Ready ? ibmPlexMono.name : "IBM Plex Mono"
    readonly property string ibmPlexSansFamily: ibmPlexSans.status === FontLoader.Ready ? ibmPlexSans.name : "IBM Plex Sans"
    readonly property string orbitronFamily: orbitron.status === FontLoader.Ready ? orbitron.name : "Orbitron"

    readonly property string rawThemeName: (client && client.themeName) ? client.themeName : ""
    readonly property string archetypeKey: ThemeCatalog.normalize(rawThemeName)
    readonly property var archetypeDef: ThemeCatalog.archetype(rawThemeName)

    // Archetype flags
    readonly property bool isNothing: archetypeKey === "nothing"
    readonly property bool isHandheld: archetypeKey === "handheld"
    readonly property bool isAmiga: archetypeKey === "amiga"
    readonly property bool isBraun: archetypeKey === "braun"
    readonly property bool isVfd: archetypeKey === "vfd_hifi"
    readonly property bool isNixie: archetypeKey === "nixie"
    readonly property bool isThinkPad: archetypeKey === "thinkpad"
    readonly property bool isUnix: archetypeKey === "unix_workstation"
    readonly property bool isC64: archetypeKey === "commodore64"
    readonly property bool isModern: archetypeKey === "modern"

    readonly property string displayName: archetypeDef.displayName || "Theme"

    // Typography
    readonly property string fontFamily: {
        switch (archetypeKey) {
        case "nothing": return geistSansFamily;
        case "handheld": return pressStartFamily;
        case "amiga": return vt323Family;
        case "braun":
        case "thinkpad": return ibmPlexSansFamily;
        case "vfd_hifi": return orbitronFamily;
        case "nixie": return ibmPlexSansFamily;
        case "unix_workstation": return ibmPlexMonoFamily;
        case "commodore64": return sysfontFamily;
        default: return "";
        }
    }

    readonly property string digitFontFamily: {
        switch (archetypeKey) {
        case "nothing": return geistMonoFamily;
        case "handheld": return pressStartFamily;
        case "amiga": return vt323Family;
        case "braun":
        case "thinkpad":
        case "unix_workstation": return ibmPlexMonoFamily;
        case "vfd_hifi": return orbitronFamily;
        case "nixie": return vt323Family;
        case "commodore64": return sysfontFamily;
        default: return "";
        }
    }

    readonly property real digitFontScale: archetypeDef.digitFontScale !== undefined ? archetypeDef.digitFontScale : 1.0

    // Palette resolution
    readonly property color accentColor: {
        if (client && client.themeAccent && client.themeAccent.length > 0) {
            return client.themeAccent;
        }
        if (archetypeDef.accentFallback && archetypeDef.accentFallback.length > 0) {
            return archetypeDef.accentFallback;
        }
        return Kirigami.Theme.highlightColor;
    }

    readonly property color secondaryAccentColor: {
        if (client && client.themeSecondaryAccent && client.themeSecondaryAccent.length > 0) {
            return client.themeSecondaryAccent;
        }
        if (archetypeDef.secondaryAccentFallback && archetypeDef.secondaryAccentFallback.length > 0) {
            return archetypeDef.secondaryAccentFallback;
        }
        return accentColor;
    }

    readonly property color textColor: {
        if (client && client.themeFg && client.themeFg.length > 0) {
            return client.themeFg;
        }
        if (archetypeDef.fgFallback && archetypeDef.fgFallback.length > 0) {
            return archetypeDef.fgFallback;
        }
        return Kirigami.Theme.textColor;
    }

    readonly property color textMutedColor: {
        if (client && client.themeTextMuted && client.themeTextMuted.length > 0) {
            return client.themeTextMuted;
        }
        if (archetypeDef.textMutedFallback && archetypeDef.textMutedFallback.length > 0) {
            return archetypeDef.textMutedFallback;
        }
        return Kirigami.Theme.disabledTextColor;
    }

    readonly property color cardBgColor: {
        if (archetypeDef.cardBg && archetypeDef.cardBg.length > 0) {
            return archetypeDef.cardBg;
        }
        return Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                       Kirigami.Theme.textColor.b, 0.04);
    }

    readonly property color cardBorderColor: {
        if (archetypeDef.cardBorder && archetypeDef.cardBorder.length > 0) {
            return archetypeDef.cardBorder;
        }
        return Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                       Kirigami.Theme.textColor.b, 0.08);
    }

    // Geometry & Surface properties
    readonly property int cardRadius: archetypeDef.cardRadius !== undefined ? archetypeDef.cardRadius : 10
    readonly property int cardBorderWidth: archetypeDef.borderWidth !== undefined ? archetypeDef.borderWidth : 1
    readonly property bool isBevel3D: archetypeDef.isBevel3D || false
    readonly property color bevelLight: archetypeDef.bevelLight || "#FFFFFF"
    readonly property color bevelDark: archetypeDef.bevelDark || "#000000"
    readonly property bool isLucent: archetypeDef.isLucent || false

    readonly property int buttonRadius: archetypeDef.buttonRadius !== undefined ? archetypeDef.buttonRadius : 6

    // Textures & Overlays
    readonly property string backgroundTexture: archetypeDef.texture && archetypeDef.texture.length > 0
        ? Qt.resolvedUrl("../../images/" + archetypeDef.texture)
        : ""
    readonly property real textureOpacity: archetypeDef.textureOpacity || 0.0
    readonly property bool hasWireMesh: archetypeDef.hasWireMesh || false
    readonly property bool hasVfdFilaments: archetypeDef.hasVfdFilaments || false

    // Glyphs, Icons & Ornaments
    readonly property string glyphStyle: archetypeDef.glyphStyle || "standard"
    readonly property string solarCurveStyle: archetypeDef.solarCurveStyle || "smooth"
    readonly property color liveIndicatorColor: archetypeDef.liveIndicatorColor || accentColor
    readonly property color burgundyAccent: archetypeDef.burgundyAccent || "#8B1D42"
    readonly property bool boingBallVisible: archetypeDef.boingBallVisible || false
    readonly property bool guruMeditationStyle: archetypeDef.guruMeditationStyle || false
}
