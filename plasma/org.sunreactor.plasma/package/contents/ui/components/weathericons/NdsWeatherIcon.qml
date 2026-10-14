/*
 * Nintendo DS weather: friendly cartoon illustrations in the handheld's menu
 * style (the weather of Animal Crossing, Nintendogs and Brain Age). Chunky
 * rounded shapes with a thick navy outline, flat saturated fills, one soft
 * highlight per shape, a slightly darker lower band, round-capped drops and
 * stubby rays.
 *
 *   NdsWeatherIcon { theme: view.theme; condition: "rain"; night: false; size: 22 }
 *
 * Shapes only, no images. The art is drawn on a 48-unit grid and scaled to
 * `size` logical px; the side is snapped to whole device pixels, so the drawing
 * lands on the same pixels at DPR 1.0, 1.35 and 2.0. The outline is an underlay:
 * each shape is stroked in navy at twice the outline width and then filled on
 * top, so only its outer half shows and overlapping shapes merge into one
 * silhouette. The outline is never thinner than 1.5 device px.
 *
 * Colours: the navy outline is the theme's text colour mixed with its DS blue,
 * the blue is the accent, the sun takes the theme's warning and secondary accent.
 * Cream, sky and the drops are fixed cartoon neutrals. Nothing animates.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    property real size: 22
    /*! Accepted for the registry contract (ThemedWeatherIcon passes it); no plate is drawn. */
    property color surface: theme.bg
    /*! The theme's text colour; the outline is mixed from it. */
    property color ink: theme.fg

    readonly property real dpr: theme.grid.devicePixelRatio > 0 ? theme.grid.devicePixelRatio : 1
    readonly property int devSize: Math.max(1, Math.round(size * dpr))
    readonly property real side: devSize / dpr
    /*! Outline width in design units: 2.6 (2.6 px at 48 px), wider when small. */
    readonly property real outline: Math.max(2.6, 77 / devSize)

    width: side
    height: side
    implicitWidth: side
    implicitHeight: side

    // --- colour --------------------------------------------------------------

    function mixc(a, b, t) {
        return Qt.rgba(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1);
    }

    readonly property color navy: mixc(root.ink, root.theme.accent, 0.3)
    readonly property color dsBlue: root.theme.accent
    readonly property color sunYellow: "#ffd24a"
    readonly property color sunOrange: root.theme.secondaryAccent
    readonly property color sunBand: mixc(sunYellow, sunOrange, 0.45)
    readonly property color cream: "#fffdf4"
    readonly property color creamBand: "#efe0b8"
    readonly property color dropColour: "#8ec9ff"
    readonly property color dropBand: mixc(dropColour, dsBlue, 0.45)
    readonly property color iceColour: "#e6f6ff"
    readonly property color mistColour: "#a9d6ff"
    readonly property color fogColour: "#f3ecd8"

    // --- which kind of picture ---------------------------------------------------

    readonly property var known: ["clear", "partly_cloudy", "cloudy", "drizzle", "rain", "heavy_rain",
                                  "thunderstorm", "snow", "mist", "atmospheric", "fog"]
    /*! Unknown conditions draw the cloud with a question mark, as the glyph does. */
    readonly property string kind: known.indexOf(condition) >= 0 ? condition : ""

    // Each table is keyed by kind. [dx, dy, scale] places the cloud on the grid.
    readonly property var sunTable: ({ "clear": [24, 24, 10], "partly_cloudy": [19, 18, 6.8] })
    readonly property var moonTable: ({ "clear": [21, 24, 11.5], "partly_cloudy": [19, 18, 7] })
    readonly property var cloudTable: ({
        "": [0, 0, 1],
        "cloudy": [0, 0, 1],
        "partly_cloudy": [5, 6, 0.82],
        "drizzle": [0, -3, 0.92],
        "rain": [0, -5, 0.92],
        "heavy_rain": [0, -5, 0.92],
        "thunderstorm": [0, -5, 0.92],
        "snow": [0, -6, 0.86]
    })
    readonly property var dropTable: ({
        "drizzle": [[17, 36, 2.3], [31, 38, 2.3]],
        "rain": [[15.5, 34, 2.8], [32.5, 34, 2.8], [24, 40, 2.8]],
        "heavy_rain": [[13, 34, 2.5], [23.5, 34, 2.5], [34, 34, 2.5], [18.5, 40.5, 2.5], [29, 40.5, 2.5]]
    })
    // Mist: thin staggered wisps. Fog: three thick bars, spaced so the outlines keep a gap.
    readonly property var barTable: ({
        "mist": [[12, 13, 34, 13], [8, 24, 40, 24], [14, 35, 30, 35]],
        "atmospheric": [[12, 13, 34, 13], [8, 24, 40, 24], [14, 35, 30, 35]],
        "fog": [[8, 12, 40, 12], [5, 24, 43, 24], [10, 36, 38, 36]]
    })
    readonly property var flakeTable: ({
        "snow": [[15.5, 35, 3], [32.5, 35, 3], [24, 40.5, 3]]
    })

    readonly property bool showSun: !night && sunTable[kind] !== undefined
    readonly property bool showMoon: night && moonTable[kind] !== undefined
    readonly property bool showStars: night && kind === "clear"
    readonly property bool showCloud: cloudTable[kind] !== undefined
    readonly property bool showDrops: dropTable[kind] !== undefined
    readonly property bool showBars: barTable[kind] !== undefined
    readonly property bool showFlakes: flakeTable[kind] !== undefined
    readonly property bool showBolt: kind === "thunderstorm"
    readonly property bool showQuestion: kind === ""

    readonly property var sunG: sunTable[kind] !== undefined ? sunTable[kind] : [24, 24, 10]
    readonly property var moonG: moonTable[kind] !== undefined ? moonTable[kind] : [24, 24, 11]
    readonly property var cloudG: cloudTable[kind] !== undefined ? cloudTable[kind] : [0, 0, 1]
    readonly property real rayW: sunG[2] * 0.38
    readonly property real barW: kind === "fog" ? 4.2 : 3
    readonly property real flakeW: 3.4

    // --- geometry: SVG path data on the 48-unit grid ----------------------------

    function f(v) {
        return Math.round(v * 1000) / 1000;
    }

    /*! A full disc, clockwise. */
    function circ(cx, cy, r) {
        return "M" + f(cx - r) + " " + f(cy)
             + "A" + f(r) + " " + f(r) + " 0 1 1 " + f(cx + r) + " " + f(cy)
             + "A" + f(r) + " " + f(r) + " 0 1 1 " + f(cx - r) + " " + f(cy) + "Z";
    }

    /*! A rounded rectangle, clockwise (the same winding as circ, so they union). */
    function rrect(x, y, w, h, r) {
        return "M" + f(x + r) + " " + f(y)
             + "H" + f(x + w - r)
             + "A" + f(r) + " " + f(r) + " 0 0 1 " + f(x + w) + " " + f(y + r)
             + "V" + f(y + h - r)
             + "A" + f(r) + " " + f(r) + " 0 0 1 " + f(x + w - r) + " " + f(y + h)
             + "H" + f(x + r)
             + "A" + f(r) + " " + f(r) + " 0 0 1 " + f(x) + " " + f(y + h - r)
             + "V" + f(y + r)
             + "A" + f(r) + " " + f(r) + " 0 0 1 " + f(x + r) + " " + f(y) + "Z";
    }

    /*! The lower segment of a disc below the height cy + k * r. */
    function segBelow(cx, cy, r, k) {
        const half = Math.sqrt(1 - k * k) * r;
        return "M" + f(cx - half) + " " + f(cy + k * r)
             + "A" + f(r) + " " + f(r) + " 0 0 0 " + f(cx + half) + " " + f(cy + k * r) + "Z";
    }

    /*! A teardrop: a round bottom of radius r centred on (cx, cy), a point above. */
    function drop(cx, cy, r) {
        const tip = cy - 2.1 * r;
        return "M" + f(cx) + " " + f(tip)
             + "C" + f(cx + 0.42 * r) + " " + f(cy - 1.3 * r) + " " + f(cx + r) + " " + f(cy - 0.7 * r) + " " + f(cx + r) + " " + f(cy)
             + "A" + f(r) + " " + f(r) + " 0 0 1 " + f(cx - r) + " " + f(cy)
             + "C" + f(cx - r) + " " + f(cy - 0.7 * r) + " " + f(cx - 0.42 * r) + " " + f(cy - 1.3 * r) + " " + f(cx) + " " + f(tip) + "Z";
    }

    /*! A four-point sparkle with concave sides. */
    function star4(cx, cy, r) {
        const k = 0.2 * r;
        return "M" + f(cx) + " " + f(cy - r)
             + "Q" + f(cx + k) + " " + f(cy - k) + " " + f(cx + r) + " " + f(cy)
             + "Q" + f(cx + k) + " " + f(cy + k) + " " + f(cx) + " " + f(cy + r)
             + "Q" + f(cx - k) + " " + f(cy + k) + " " + f(cx - r) + " " + f(cy)
             + "Q" + f(cx - k) + " " + f(cy - k) + " " + f(cx) + " " + f(cy - r) + "Z";
    }

    /*! A crescent: the disc (cx, cy, r) minus the disc offset by (ox, oy) with radius ro. */
    function crescent(cx, cy, r, ox, oy, ro) {
        const d = Math.hypot(ox, oy);
        const a = (r * r - ro * ro + d * d) / (2 * d);
        const h = Math.sqrt(r * r - a * a);
        const ux = ox / d, uy = oy / d;
        const mx = cx + a * ux, my = cy + a * uy;
        const px = mx - h * uy, py = my + h * ux;
        const qx = mx + h * uy, qy = my - h * ux;
        return "M" + f(px) + " " + f(py)
             + "A" + f(r) + " " + f(r) + " 0 " + (a > 0 ? 1 : 0) + " 1 " + f(qx) + " " + f(qy)
             + "A" + f(ro) + " " + f(ro) + " 0 " + (d - a > 0 ? 0 : 1) + " 0 " + f(px) + " " + f(py) + "Z";
    }

    /*! Straight strokes from radius r0 to r1 at n evenly spaced angles. */
    function rays(cx, cy, r0, r1, n) {
        let s = "";
        for (let i = 0; i < n; ++i) {
            const a = 2 * Math.PI * i / n;
            s += "M" + f(cx + r0 * Math.cos(a)) + " " + f(cy + r0 * Math.sin(a))
               + "L" + f(cx + r1 * Math.cos(a)) + " " + f(cy + r1 * Math.sin(a));
        }
        return s;
    }

    /*! Three strokes through the centre: six round-ended arms. */
    function flake(cx, cy, r) {
        const dx = 0.866 * r, dy = 0.5 * r;
        return "M" + f(cx) + " " + f(cy - r) + "L" + f(cx) + " " + f(cy + r)
             + "M" + f(cx - dx) + " " + f(cy - dy) + "L" + f(cx + dx) + " " + f(cy + dy)
             + "M" + f(cx - dx) + " " + f(cy + dy) + "L" + f(cx + dx) + " " + f(cy - dy);
    }

    function poly(points) {
        let s = "";
        points.forEach((p, i) => {
            s += (i ? "L" : "M") + f(p[0]) + " " + f(p[1]);
        });
        return s + "Z";
    }

    function bar(b) {
        return "M" + f(b[0]) + " " + f(b[1]) + "L" + f(b[2]) + " " + f(b[3]);
    }

    function each(list, fn) {
        return (list || []).map(fn).join("");
    }

    // The cloud: three lobes and a flat base, placed by the cloud table.
    function cloudX(x) { return 24 + (x - 24) * root.cloudG[2] + root.cloudG[0]; }
    function cloudY(y) { return 24 + (y - 24) * root.cloudG[2] + root.cloudG[1]; }

    readonly property string sunD: circ(sunG[0], sunG[1], sunG[2])
    readonly property string sunRaysD: rays(sunG[0], sunG[1], sunG[2] * 1.3, sunG[2] * 1.72, 8)
    readonly property string sunBandD: segBelow(sunG[0], sunG[1], sunG[2], 0.3)
    readonly property string sunGlintD: circ(sunG[0] - 0.4 * sunG[2], sunG[1] - 0.4 * sunG[2], 0.26 * sunG[2])
    readonly property real sunGlintX: sunG[0] - 0.4 * sunG[2]
    readonly property real sunGlintY: sunG[1] - 0.4 * sunG[2]

    readonly property string moonD: crescent(moonG[0], moonG[1], moonG[2],
                                             0.6 * moonG[2], -0.5 * moonG[2], 0.85 * moonG[2])
    readonly property string moonGlintD: circ(moonG[0] - 0.45 * moonG[2], moonG[1] - 0.25 * moonG[2], 0.2 * moonG[2])
    readonly property real moonGlintX: moonG[0] - 0.45 * moonG[2]
    readonly property real moonGlintY: moonG[1] - 0.25 * moonG[2]

    readonly property string starsD: star4(36, 13, 4.6) + star4(42, 24, 2.8) + star4(33, 37, 3.2)

    readonly property string cloudD: circ(cloudX(15.5), cloudY(27), 7.5 * cloudG[2])
                                   + circ(cloudX(23.5), cloudY(20.5), 9.5 * cloudG[2])
                                   + circ(cloudX(33.5), cloudY(27), 6.5 * cloudG[2])
                                   + rrect(cloudX(8), cloudY(26.5), 32 * cloudG[2], 10 * cloudG[2], 5 * cloudG[2])
    readonly property string cloudBandD: rrect(cloudX(12.5), cloudY(30.5), 23 * cloudG[2], 6 * cloudG[2], 3 * cloudG[2])
    readonly property string cloudGlintD: circ(cloudX(18.5), cloudY(16.5), 3.2 * cloudG[2])
    readonly property real cloudGlintX: cloudX(18.5)
    readonly property real cloudGlintY: cloudY(16.5)
    readonly property real cloudGlintR: 3.6 * cloudG[2]

    readonly property string dropsD: each(dropTable[kind], d => drop(d[0], d[1], d[2]))
    readonly property string dropBandD: each(dropTable[kind], d => segBelow(d[0], d[1], d[2], 0.3))
    readonly property string dropDotsD: each(dropTable[kind], d => circ(d[0] - 0.4 * d[2], d[1] - 0.25 * d[2], 0.3 * d[2]))

    readonly property string barsD: each(barTable[kind], bar)
    readonly property string flakesD: each(flakeTable[kind], k => flake(k[0], k[1], k[2]))
    readonly property string boltD: poly([[28, 19], [17, 33], [24, 33], [20, 45], [32, 28], [25, 28], [30, 19]])
    readonly property string questionD: "M20.5 19.5Q20.5 15 24 15Q27.5 15 27.5 18.5Q27.5 21 24 23L24 26.5"
    readonly property string questionDotD: circ(24, 30.5, 1.6)

    // --- paint -----------------------------------------------------------------

    /*! One fixed piece of paint: a fill, a stroke, or both (set one of them). */
    component Part: ShapePath {
        id: part

        property bool shown: true
        property string d: ""
        property color fillC: "transparent"
        property color strokeC: "transparent"
        property real strokeW: 0

        strokeColor: strokeC
        strokeWidth: strokeW
        fillColor: fillC
        /*! Overlapping lobes must union, not cancel (the default is odd-even). */
        fillRule: ShapePath.WindingFill
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg { path: part.shown ? part.d : "" }
    }

    /*! The soft highlight: a white blob fading to nothing at its edge. */
    component Glint: ShapePath {
        id: glint

        property bool shown: true
        property string d: ""
        property real gx: 0
        property real gy: 0
        property real gr: 1

        strokeColor: "transparent"
        fillColor: "transparent"
        fillGradient: RadialGradient {
            centerX: glint.gx
            centerY: glint.gy
            focalX: glint.gx
            focalY: glint.gy
            centerRadius: glint.gr
            GradientStop { position: 0; color: "#73ffffff" }
            GradientStop { position: 1; color: "#00ffffff" }
        }

        PathSvg { path: glint.shown ? glint.d : "" }
    }

    Shape {
        id: art

        width: 48
        height: 48
        x: 0
        y: 0
        scale: root.side / 48
        transformOrigin: Item.TopLeft
        antialiasing: true
        preferredRendererType: Shape.CurveRenderer

        // Back to front: night stars, the sun or moon, drops, bars, flakes, cloud, bolt, question mark.
        Part { shown: root.showStars; d: root.starsD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showStars; d: root.starsD; fillC: root.sunYellow }

        Part { shown: root.showSun; d: root.sunRaysD; strokeC: root.navy; strokeW: root.rayW + 2 * root.outline }
        Part { shown: root.showSun; d: root.sunRaysD; strokeC: root.sunOrange; strokeW: root.rayW }
        Part { shown: root.showSun; d: root.sunD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showSun; d: root.sunD; fillC: root.sunYellow }
        Part { shown: root.showSun; d: root.sunBandD; fillC: root.sunBand }
        Glint { shown: root.showSun; d: root.sunGlintD; gx: root.sunGlintX; gy: root.sunGlintY; gr: 0.3 * root.sunG[2] }

        Part { shown: root.showMoon; d: root.moonD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showMoon; d: root.moonD; fillC: root.sunYellow }
        Glint { shown: root.showMoon; d: root.moonGlintD; gx: root.moonGlintX; gy: root.moonGlintY; gr: 0.3 * root.moonG[2] }

        Part { shown: root.showDrops; d: root.dropsD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showDrops; d: root.dropsD; fillC: root.dropColour }
        Part { shown: root.showDrops; d: root.dropBandD; fillC: root.dropBand }
        Part { shown: root.showDrops; d: root.dropDotsD; fillC: Qt.rgba(1, 1, 1, 0.8) }

        Part { shown: root.showBars; d: root.barsD; strokeC: root.navy; strokeW: root.barW + 2 * root.outline }
        Part { shown: root.showBars; d: root.barsD; strokeC: root.kind === "fog" ? root.fogColour : root.mistColour; strokeW: root.barW }

        Part { shown: root.showFlakes; d: root.flakesD; strokeC: root.navy; strokeW: root.flakeW + 2 * root.outline }
        Part { shown: root.showFlakes; d: root.flakesD; strokeC: root.iceColour; strokeW: root.flakeW }

        Part { shown: root.showCloud; d: root.cloudD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showCloud; d: root.cloudD; fillC: root.cream }
        Part { shown: root.showCloud; d: root.cloudBandD; fillC: root.creamBand }
        Glint { shown: root.showCloud; d: root.cloudGlintD; gx: root.cloudGlintX; gy: root.cloudGlintY; gr: root.cloudGlintR }

        Part { shown: root.showBolt; d: root.boltD; strokeC: root.navy; strokeW: 2 * root.outline }
        Part { shown: root.showBolt; d: root.boltD; fillC: root.sunYellow }

        Part { shown: root.showQuestion; d: root.questionD; strokeC: root.navy; strokeW: 2.6 }
        Part { shown: root.showQuestion; d: root.questionDotD; fillC: root.navy }
    }
}
