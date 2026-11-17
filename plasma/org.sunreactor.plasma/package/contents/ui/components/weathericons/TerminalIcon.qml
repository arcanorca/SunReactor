/*
 * Terminal family (dracula, the terminal skin's themes): a 16x16 (or 11x11, 6x6) pixel sky in
 * the TUI's own roles (sun/bolt in the warning colour, rain in the secondary accent,
 * fog in the muted ink, the rest in the ink), drawn as whole device-pixel squares.
 * Smaller squares use a dedicated 11x11 or 6x6 sprite so the art always fills the square.
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
    /*! Sprite grid: the largest whose whole-pixel cells fill 78% of the square, else the best fill. */
    readonly property int gridCells: {
        const d = size * dpr;
        let best = 6;
        let bestFill = 0;
        for (const g of [16, 11, 6]) {
            const a = Math.floor(d / g + 0.001);
            if (a < 1) {
                continue;
            }
            const fill = g * a / d;
            if (fill >= 0.78) {
                return g;
            }
            if (fill > bestFill) {
                bestFill = fill;
                best = g;
            }
        }
        return best;
    }
    /*! Device px per art px: whole device pixels, centred in the square. */
    readonly property int artPx: Math.max(1, Math.floor(size * dpr / gridCells + 0.001))
    readonly property int originX: Math.round((width * dpr - gridCells * artPx) / 2)
    readonly property int originY: Math.round((height * dpr - gridCells * artPx) / 2)

    readonly property string artKey: (night && (condition === "clear" || condition === "partly_cloudy"))
        ? "night_" + condition
        : condition
    readonly property var activeTable: gridCells === 16 ? artTable16 : gridCells === 11 ? artTable11 : artTable6
    readonly property var art: activeTable[artKey] || activeTable.unknown

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

    readonly property var artTable16: ({
        clear: [
            ".......SS.......",
            ".......SS.......",
            "..S....SS....S..",
            "...S........S...",
            "......SSSS......",
            ".....SSSSSS.....",
            "....SSSSSSSS....",
            "SS..SSSSSSSS..SS",
            "SS..SSSSSSSS..SS",
            "....SSSSSSSS....",
            ".....SSSSSS.....",
            "......SSSS......",
            "...S........S...",
            "..S....SS....S..",
            ".......SS.......",
            ".......SS......."
        ],
        night_clear: [
            "................",
            ".......MMMM.....",
            "....MMMMMMM..#..",
            "...MMMMMM...###.",
            "..MMMMM......#..",
            "..MMMM..........",
            ".MMMM...........",
            ".MMMM...........",
            ".MMMM.........#.",
            ".MMMM...........",
            "..MMMM......#...",
            "..MMMMM....###..",
            "...MMMMMM...#...",
            "....MMMMMMM.....",
            ".......MMMM.....",
            "................"
        ],
        partly_cloudy: [
            ".....S..........",
            ".S...S...S......",
            "................",
            "...SSSSS........",
            "..SSSSSSS.......",
            "S.SSSSSSS.S.....",
            "..SSSSSSS.......",
            "...SSSSS........",
            "................",
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############."
        ],
        night_partly_cloudy: [
            "...MMMM.........",
            "..MMMM..........",
            ".MMMM........#..",
            ".MMMM.......###.",
            ".MMMM........#..",
            ".MMMM...........",
            "..MMMM..........",
            "...MMMM....#..#.",
            "................",
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############."
        ],
        cloudy: [
            "................",
            "................",
            "........FFF.....",
            ".......FFFFF.FF.",
            "......FFFFFFFFFF",
            ".....FFFFFFFFFFF",
            ".....FFFFFFFFFFF",
            "........FFFFFFF.",
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................"
        ],
        drizzle: [
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................",
            "................",
            "....R...R...R...",
            "................",
            "................",
            "..R...R...R...R.",
            "................",
            "................",
            "....R...R...R..."
        ],
        rain: [
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................",
            "....R...R...R...",
            "...R...R...R....",
            "................",
            "..R...R...R...R.",
            ".R...R...R...R..",
            "................",
            "....R...R...R...",
            "...R...R...R...."
        ],
        heavy_rain: [
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................",
            "...R..R..R..R...",
            "..R..R..R..R....",
            ".R..R..R..R.....",
            "................",
            "..R..R..R..R..R.",
            ".R..R..R..R..R..",
            "R..R..R..R..R...",
            "................"
        ],
        thunderstorm: [
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................",
            "........BBB.....",
            "..R....BBB....R.",
            ".R....BBB....R..",
            ".....BBBBBB.....",
            ".......BBB......",
            "...R..BBB....R..",
            "..R..BBB....R...",
            ".....BB........."
        ],
        snow: [
            "....####........",
            "...######..###..",
            "..#######.#####.",
            ".###############",
            "################",
            "################",
            ".##############.",
            "................",
            "...#....#....#..",
            "..###..###..###.",
            "...#....#....#..",
            "................",
            ".....#.....#....",
            "....###...###...",
            ".....#.....#....",
            "................"
        ],
        mist: [
            "................",
            "FFFFFFFFFFFFFFFF",
            "................",
            "................",
            "..FFFFFFFFFFFF..",
            "................",
            "................",
            "FFFFFFFFFFFFFFFF",
            "................",
            "................",
            "..FFFFFFFFFFFF..",
            "................",
            "................",
            "FFFFFFFFFFFFFFFF",
            "................",
            "................"
        ],
        fog: [
            "................",
            ".FFFFFFFFFFFFFF.",
            ".FFFFFFFFFFFFFF.",
            "................",
            "..FFFFFFFFFFFF..",
            "..FFFFFFFFFFFF..",
            "................",
            ".FFFFFFFFFFFFFF.",
            ".FFFFFFFFFFFFFF.",
            "................",
            "..FFFFFFFFFFFF..",
            "..FFFFFFFFFFFF..",
            "................",
            ".FFFFFFFFFFFFFF.",
            ".FFFFFFFFFFFFFF.",
            "................"
        ],
        atmospheric: [
            "................",
            "FFFF..FFFF..FFFF",
            "................",
            "................",
            ".F..FFFF..FFFF..",
            "................",
            "................",
            "FFFF..FFFF..FFFF",
            "................",
            "................",
            ".F..FFFF..FFFF..",
            "................",
            "................",
            "FFFF..FFFF..FFFF",
            "................",
            "................"
        ],
        unknown: [
            "................",
            "....########....",
            "...##########...",
            "..####....####..",
            "..###......###..",
            "...........###..",
            "..........###...",
            ".........###....",
            "........###.....",
            ".......###......",
            ".......###......",
            "................",
            "................",
            ".......###......",
            ".......###......",
            "................"
        ]
    })

    readonly property var artTable11: ({
        clear: [
            ".....S.....",
            ".S...S...S.",
            "...........",
            "...SSSSS...",
            "..SSSSSSS..",
            "S.SSSSSSS.S",
            "..SSSSSSS..",
            "...SSSSS...",
            "...........",
            ".S...S...S.",
            ".....S....."
        ],
        night_clear: [
            "...........",
            "...MMMM..#.",
            "..MMMMM.###",
            ".MMMM....#.",
            ".MMM.......",
            ".MMM.......",
            ".MMM....#..",
            ".MMMM......",
            "..MMMMM..#.",
            "...MMMM....",
            "..........."
        ],
        partly_cloudy: [
            ".S.S.S.....",
            "..SSS......",
            "S.SSS.S....",
            "..SSS......",
            "...........",
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########."
        ],
        night_partly_cloudy: [
            ".MMM.......",
            "MM......#..",
            "MM.....###.",
            "MM......#..",
            ".MM...#....",
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########."
        ],
        cloudy: [
            "...........",
            "...........",
            "...........",
            "...........",
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            "..........."
        ],
        drizzle: [
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            "...........",
            "...........",
            "...R...R...",
            "...........",
            "..R..R..R.."
        ],
        rain: [
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            "...........",
            "...R..R..R.",
            "..R..R..R..",
            "..R..R..R..",
            ".R..R..R..."
        ],
        heavy_rain: [
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            "...........",
            "...R.R.R.R.",
            "..R.R.R.R..",
            "..R.R.R.R.R",
            ".R.R.R.R.R."
        ],
        thunderstorm: [
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            ".....BB....",
            "..R.BB...R.",
            ".R.BBBB.R..",
            ".....BB....",
            "....BB....."
        ],
        snow: [
            "...###.....",
            "..#####.##.",
            ".##########",
            "###########",
            "###########",
            ".#########.",
            "...........",
            "...#...#...",
            "..###.###..",
            "...#...#...",
            ".#...#...#."
        ],
        mist: [
            "FFFFFFFFFFF",
            "...........",
            ".FFFFFFFFF.",
            "...........",
            "FFFFFFFFFFF",
            "...........",
            ".FFFFFFFFF.",
            "...........",
            "FFFFFFFFFFF",
            "...........",
            ".FFFFFFFFF."
        ],
        fog: [
            "FFFFFFFFFFF",
            "FFFFFFFFFFF",
            "...........",
            ".FFFFFFFFF.",
            ".FFFFFFFFF.",
            "...........",
            "FFFFFFFFFFF",
            "FFFFFFFFFFF",
            "...........",
            ".FFFFFFFFF.",
            ".FFFFFFFFF."
        ],
        atmospheric: [
            "FFFF..FFFF.",
            "...........",
            "F..FFFF..FF",
            "...........",
            "FFFF..FFFF.",
            "...........",
            "F..FFFF..FF",
            "...........",
            "FFFF..FFFF.",
            "...........",
            "F..FFFF..FF"
        ],
        unknown: [
            "...#####...",
            "..##...##..",
            "..##...##..",
            ".......##..",
            "......##...",
            ".....##....",
            ".....##....",
            "...........",
            ".....##....",
            ".....##....",
            "..........."
        ]
    })

    readonly property var artTable6: ({
        clear: ["S.SS.S", ".S..S.", "SS..SS", "SS..SS", ".S..S.", "S.SS.S"],
        night_clear: ["..MMM.", ".MM...", ".M....", ".M....", ".MM...", "..MMM."],
        partly_cloudy: ["S.S...", ".S....", "S.S##.", "..####", ".#####", "......"],
        night_partly_cloudy: [".MM...", "M.....", "M..##.", "..####", ".#####", "......"],
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
