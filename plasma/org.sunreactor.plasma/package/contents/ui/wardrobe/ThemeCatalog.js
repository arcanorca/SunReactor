/*
 * SPDX-FileCopyrightText: 2026 SunReactor Contributors
 * SPDX-License-Identifier: MIT
 */

.pragma library

var ARCHETYPES = {
    nothing: {
        id: "nothing",
        displayName: "Nothing OS",
        cardRadius: 16,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: true,
        glyphStyle: "dotmatrix",
        solarCurveStyle: "dotmatrix",
        fontFamily: "Geist",
        digitFontFamily: "Geist Mono",
        texture: "",
        textureOpacity: 0.0,
        liveIndicatorColor: "#E50914",
        accentFallback: "#FFFFFF",
        secondaryAccentFallback: "#E50914",
        cardBg: "rgba(18, 18, 22, 0.70)",
        cardBorder: "rgba(255, 255, 255, 0.12)",
        fgFallback: "#FFFFFF",
        textMutedFallback: "#888888",
        buttonRadius: 8
    },
    handheld: {
        id: "handheld",
        displayName: "Handheld DMG-01",
        cardRadius: 2,
        borderWidth: 2,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "pixel",
        solarCurveStyle: "pixel",
        fontFamily: "Press Start 2P",
        digitFontFamily: "Press Start 2P",
        texture: "gameboy-dmg.svg",
        textureOpacity: 0.08,
        liveIndicatorColor: "#C0392B",
        burgundyAccent: "#8B1D42",
        accentFallback: "#8B1D42",
        secondaryAccentFallback: "#306230",
        cardBg: "#8BAC0F",
        cardBorder: "#0F380F",
        fgFallback: "#0F380F",
        textMutedFallback: "#306230",
        buttonRadius: 2,
        digitFontScale: 0.82
    },
    amiga: {
        id: "amiga",
        displayName: "Amiga Workbench",
        cardRadius: 0,
        borderWidth: 2,
        isBevel3D: true,
        isLucent: false,
        glyphStyle: "bevel",
        solarCurveStyle: "smooth",
        fontFamily: "VT323",
        digitFontFamily: "VT323",
        texture: "amiga-boing-ball.svg",
        textureOpacity: 0.06,
        bevelLight: "#FFFFFF",
        bevelDark: "#000000",
        liveIndicatorColor: "#FF8800",
        guruMeditationStyle: true,
        boingBallVisible: true,
        accentFallback: "#FF8800",
        secondaryAccentFallback: "#5588BB",
        cardBg: "#5588BB",
        cardBorder: "#000000",
        fgFallback: "#FFFFFF",
        textMutedFallback: "#DDEEFF",
        buttonRadius: 0,
        digitFontScale: 1.15
    },
    braun: {
        id: "braun",
        displayName: "Braun ET66",
        cardRadius: 8,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "industrial",
        solarCurveStyle: "smooth",
        fontFamily: "IBM Plex Sans",
        digitFontFamily: "IBM Plex Mono",
        texture: "statoscope-reticle-dark-tile.svg",
        textureOpacity: 0.04,
        buttonRadius: 100,
        liveIndicatorColor: "#FF5500",
        accentFallback: "#FF5500",
        secondaryAccentFallback: "#F0B01E",
        cardBg: "#1C1C1E",
        cardBorder: "#2E2E32",
        fgFallback: "#EEEEEE",
        textMutedFallback: "#8E8E93"
    },
    vfd_hifi: {
        id: "vfd_hifi",
        displayName: "VFD Hi-Fi",
        cardRadius: 4,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "fluorescent",
        solarCurveStyle: "vfd",
        fontFamily: "Orbitron",
        digitFontFamily: "Orbitron",
        texture: "vfd-grid-tile.svg",
        textureOpacity: 0.22,
        hasVfdFilaments: true,
        liveIndicatorColor: "#00F0A8",
        accentFallback: "#00F0A8",
        secondaryAccentFallback: "#FFB000",
        cardBg: "#070E0B",
        cardBorder: "#00F0A8",
        fgFallback: "#66FFD4",
        textMutedFallback: "#008A60",
        buttonRadius: 4,
        digitFontScale: 0.90
    },
    nixie: {
        id: "nixie",
        displayName: "Nixie Tubes",
        cardRadius: 8,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "neon",
        solarCurveStyle: "neon",
        fontFamily: "IBM Plex Sans",
        digitFontFamily: "VT323",
        texture: "nixie-mesh-tile.svg",
        textureOpacity: 0.35,
        hasWireMesh: true,
        liveIndicatorColor: "#FF7A18",
        accentFallback: "#FF7A18",
        secondaryAccentFallback: "#FFB765",
        cardBg: "#120D0A",
        cardBorder: "#3D2413",
        fgFallback: "#FF9E4A",
        textMutedFallback: "#8C5E3D",
        buttonRadius: 6,
        digitFontScale: 1.15
    },
    thinkpad: {
        id: "thinkpad",
        displayName: "ThinkPad Raven",
        cardRadius: 3,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "industrial",
        solarCurveStyle: "smooth",
        fontFamily: "IBM Plex Sans",
        digitFontFamily: "IBM Plex Mono",
        texture: "",
        textureOpacity: 0.0,
        liveIndicatorColor: "#DA291C",
        accentFallback: "#DA291C",
        secondaryAccentFallback: "#303030",
        cardBg: "#141416",
        cardBorder: "#2E2E32",
        fgFallback: "#F0F0F0",
        textMutedFallback: "#777777",
        buttonRadius: 3
    },
    unix_workstation: {
        id: "unix_workstation",
        displayName: "UNIX Workstation",
        cardRadius: 0,
        borderWidth: 2,
        isBevel3D: true,
        isLucent: false,
        glyphStyle: "bevel",
        solarCurveStyle: "smooth",
        fontFamily: "IBM Plex Mono",
        digitFontFamily: "IBM Plex Mono",
        texture: "",
        textureOpacity: 0.0,
        bevelLight: "#8D9EA8",
        bevelDark: "#262E34",
        liveIndicatorColor: "#00FF7F",
        accentFallback: "#4580A0",
        secondaryAccentFallback: "#8D9EA8",
        cardBg: "#4E5D6C",
        cardBorder: "#262E34",
        fgFallback: "#FFFFFF",
        textMutedFallback: "#A5B5C0",
        buttonRadius: 0
    },
    commodore64: {
        id: "commodore64",
        displayName: "Commodore 64",
        cardRadius: 0,
        borderWidth: 2,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "pixel",
        solarCurveStyle: "pixel",
        fontFamily: "Sysfont",
        digitFontFamily: "Sysfont",
        texture: "",
        textureOpacity: 0.0,
        liveIndicatorColor: "#A0A0FF",
        accentFallback: "#8870E6",
        secondaryAccentFallback: "#A0A0FF",
        cardBg: "#40318D",
        cardBorder: "#8870E6",
        fgFallback: "#A0A0FF",
        textMutedFallback: "#6C5EB5",
        buttonRadius: 0,
        digitFontScale: 0.95
    },
    modern: {
        id: "modern",
        displayName: "Modern GNOME HIG",
        cardRadius: 10,
        borderWidth: 1,
        isBevel3D: false,
        isLucent: false,
        glyphStyle: "standard",
        solarCurveStyle: "smooth",
        fontFamily: "",
        digitFontFamily: "",
        texture: "",
        textureOpacity: 0.0,
        liveIndicatorColor: "",
        accentFallback: "",
        secondaryAccentFallback: "",
        cardBg: "",
        cardBorder: "",
        fgFallback: "",
        textMutedFallback: "",
        buttonRadius: 6,
        digitFontScale: 1.0
    }
};

var ALIASES = {
    nothing: "nothing", glyph: "nothing", nothing_glyph: "nothing", nothingos: "nothing",
    handheld: "handheld", gameboy: "handheld", dmg: "handheld", dmg01: "handheld", pocket: "handheld",
    amiga: "amiga", workbench: "amiga", amiga500: "amiga", amiga1200: "amiga",
    braun: "braun", din: "braun", et66: "braun", rams: "braun",
    vfd_hifi: "vfd_hifi", vfd_hi_fi: "vfd_hifi", vfdhifi: "vfd_hifi", vfd: "vfd_hifi", hifi: "vfd_hifi",
    nixie: "nixie", tubes: "nixie", nixie_tubes: "nixie", cold_cathode: "nixie",
    thinkpad: "thinkpad", ibm: "thinkpad", trackpoint: "thinkpad",
    unix_workstation: "unix_workstation", unixworkstation: "unix_workstation", unix: "unix_workstation",
    workstation: "unix_workstation", motif: "unix_workstation", irix: "unix_workstation",
    cde: "unix_workstation", sgi: "unix_workstation", solaris: "unix_workstation",
    commodore64: "commodore64", c64: "commodore64",
    oscilloscope: "vfd_hifi", scope: "vfd_hifi", crt: "commodore64",
    casiodigital: "handheld", casio: "handheld", digitalwatch: "handheld", digital_watch: "handheld",
    classicmacintosh: "braun", mac128k: "braun", macintosh128k: "braun"
};

function normalize(name) {
    if (!name) return "modern";
    var raw = String(name).trim().toLowerCase();
    var key = raw.replace(/[-_ ]+/g, "_");
    if (ARCHETYPES[key] !== undefined) return key;
    var alias = ALIASES[key];
    if (alias !== undefined && ARCHETYPES[alias] !== undefined) return alias;
    var stripped = raw.replace(/[^a-z0-9]/g, "");
    if (ARCHETYPES[stripped] !== undefined) return stripped;
    if (ALIASES[stripped] !== undefined && ARCHETYPES[ALIASES[stripped]] !== undefined) {
        return ALIASES[stripped];
    }
    // Suffix/prefix checks with minimum token length to prevent substring accidents
    for (var a in ALIASES) {
        if (a.length >= 3 && (key.indexOf(a) !== -1 || stripped.indexOf(a) !== -1)) {
            return ALIASES[a];
        }
    }
    return "modern";
}

function archetype(name) {
    return ARCHETYPES[normalize(name)];
}
