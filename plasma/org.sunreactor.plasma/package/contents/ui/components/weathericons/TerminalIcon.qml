/*
 * Terminal family (dracula, the terminal skin's themes): a six-by-six pixel sky in
 * the TUI's own roles (sun in the warning colour, rain in the secondary accent,
 * fog in the muted ink, the rest in the ink), drawn as whole device-pixel squares.
 * Terminal cells cannot carry legible ASCII at panel size, so this is the block art
 * the TUI draws, one colour per pixel instead of half-block pairs.
 *
 * Letters: S sun, M moon, B bolt, R rain, F fog, # cloud and snow (the ink), . off.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../panel/PanelData.js" as PanelData

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    property real size: 22
    property color surface: theme.bg
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    readonly property real dpr: theme.grid.devicePixelRatio
    /*! Device px per art px: six art px across the icon, whole device pixels. */
    readonly property int artPx: Math.max(1, Math.floor(size * dpr / 6 + 0.001))
    readonly property int originX: Math.round((width * dpr - 6 * artPx) / 2)
    readonly property int originY: Math.round((height * dpr - 6 * artPx) / 2)

    readonly property var art: artTable[(condition === "clear" && night) ? "night_clear" : condition] || artTable.unknown

    /*! Every lit pixel as { x, y, c } (art px, and its colour role). */
    readonly property var pixels: {
        const out = [];
        for (let y = 0; y < art.length; ++y) {
            for (let x = 0; x < art[y].length; ++x) {
                const role = art[y].charAt(x);
                if (role !== ".") {
                    out.push({ x: x, y: y, c: roleColour(role) });
                }
            }
        }
        return out;
    }

    function roleColour(role) {
        switch (role) {
        case "S":
        case "B":
            return PanelData.ink(theme.warning, surface, ink, 3);
        case "R":
            return PanelData.ink(theme.secondaryAccent, surface, ink, 3);
        case "F":
            return PanelData.ink(theme.textMuted, surface, ink, 3);
        default:
            return ink;
        }
    }

    readonly property var artTable: ({
        clear: ["S.SS.S", ".S..S.", "SS..SS", "SS..SS", ".S..S.", "S.SS.S"],
        night_clear: ["..MMM.", ".MM...", ".M....", ".M....", ".MM...", "..MMM."],
        partly_cloudy: ["S.S...", ".S....", "S.S##.", "..####", ".#####", "......"],
        cloudy: ["......", "..##..", ".####.", "######", "######", "......"],
        drizzle: ["......", "..##..", ".####.", "######", ".R..R.", "R..R.."],
        rain: ["......", "..##..", ".####.", "######", "R.R.R.", ".R.R.R"],
        heavy_rain: ["......", "..##..", ".####.", "######", "RR.RR.", "R.RR.R"],
        thunderstorm: ["......", "..##..", ".####.", "######", ".BB...", "..B..."],
        snow: ["......", "..##..", ".####.", "######", "#.#.#.", ".#.#.#"],
        mist: ["......", "FFFFFF", "......", "FFFFFF", "......", "FFFFFF"],
        fog: ["......", "FFFFFF", "......", "FFFFFF", "......", "FFFFFF"],
        atmospheric: ["......", "FFFFFF", "......", "FFFFFF", "......", "FFFFFF"],
        unknown: [".####.", "##..##", "...##.", "..##..", "......", "..##.."]
    })

    Repeater {
        model: root.pixels

        delegate: Rectangle {
            required property var modelData

            x: (root.originX + modelData.x * root.artPx) / root.dpr
            y: (root.originY + modelData.y * root.artPx) / root.dpr
            width: root.artPx / root.dpr
            height: root.artPx / root.dpr
            color: modelData.c
            antialiasing: false
        }
    }
}
