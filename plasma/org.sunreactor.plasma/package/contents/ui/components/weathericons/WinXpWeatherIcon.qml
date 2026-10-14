/*
 * Windows XP (Luna) weather icons, the MSN Weather / Windows Live Messenger look
 * of 2001-2007: glossy vector art on a 48 x 48 design grid, scaled to `size`
 * logical px and laid out in whole device pixels. Each condition is built from
 * the same parts: a sun or a crescent moon, a Luna cloud, then whatever the
 * weather adds (drops, a bolt, crystals, fog bars, or the help balloon for an
 * unknown reading). Every part has a 1 device px steel outline, a gradient fill
 * and a white gloss on its upper left, and a static shadow ellipse sits under it.
 *
 * The palette is the Luna one, fixed: the icons keep their look under every theme.
 * On a dark surface (the panel) a faint white halo sits behind the art, so the
 * steel outlines still read; on a light surface there is no halo.
 *
 *   WinXpWeatherIcon { theme: view.theme; condition: "rain"; night: false; size: 22 }
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    required property string condition
    required property bool night
    required property var theme
    /*! Logical px, square. */
    property real size: 22
    /*! What the icon sits on: a dark surface gets the halo. */
    property color surface: theme.bg
    /*! Kept for the weather icon contract; the Luna palette does not follow the theme. */
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    // ---- device pixels ------------------------------------------------------------

    readonly property real dpr: theme.grid.devicePixelRatio
    readonly property int devSize: Math.max(1, Math.round(size * dpr))
    /*! The side in logical px, a whole number of device pixels. */
    readonly property real side: devSize / dpr
    /*! Logical px per design unit. */
    readonly property real unit: side / design
    /*! One device px in design units: the outline width. */
    readonly property real hair: design / devSize
    readonly property real design: 48
    readonly property bool halo: theme.luminance(surface) < 0.3

    // ---- what the condition draws ---------------------------------------------------

    readonly property bool partly: condition === "partly_cloudy"
    readonly property bool cloudy: condition === "cloudy"
    readonly property bool sunny: !night && (condition === "clear" || partly)
    readonly property bool moony: night && (condition === "clear" || partly)
    readonly property bool hasCloud: partly || cloudy
        || ["drizzle", "rain", "heavy_rain", "thunderstorm", "snow"].indexOf(condition) >= 0
    readonly property bool fog: ["mist", "atmospheric", "fog"].indexOf(condition) >= 0
    readonly property bool help: !sunny && !moony && !hasCloud && !fog

    /*! Sun: centred for a clear sky, upper left when a cloud sits in front of it. */
    readonly property real sunC: partly ? 18 : 24
    readonly property real sunR: partly ? 9 : 11.5
    /*! Front cloud: the lower part of the sky, or the whole sky for a storm or snow. */
    readonly property real cloudX: cloudy ? 2 : 4
    readonly property real cloudY: partly || cloudy ? 14 : 0

    // ---- Luna colours ---------------------------------------------------------------

    readonly property color steel: "#5a6e8c"
    readonly property color navy: "#1b3a6b"
    readonly property color gloss: Qt.rgba(1, 1, 1, 0.92)

    // ---- geometry (design units) ---------------------------------------------------

    /*! Crescent moons: the outer disc minus an offset disc, one closed path each. */
    readonly property string moonDay: "M 21.65 11.21 A 13 13 0 1 0 36.98 24.63 A 10.5 10.5 0 0 1 21.65 11.21 Z"
    readonly property string moonPartly: "M 14.92 6.57 A 8.5 8.5 0 1 0 24.43 16.08 A 7.2 7.2 0 0 1 14.92 6.57 Z"

    /*! The cloud outline in its own units (40 x 19, flat base on y = 23): four
        bumps on a base, arcs to the outward intersections of neighbouring circles. */
    readonly property var cloudOps: [
        ["M", 8, 23], ["L", 37, 23],
        ["A", 5, 0, 0, 38.0, 13.1],
        ["A", 9, 0, 0, 22.79, 6.48],
        ["A", 8, 0, 0, 9.05, 11.09],
        ["A", 6, 1, 0, 8, 23]
    ]

    function fmt(v) {
        return Math.round(v * 100) / 100;
    }

    /*! The cloud outline placed at (x0, y0), scaled by s. */
    function cloudPath(x0, y0, s) {
        let d = "";
        for (const op of cloudOps) {
            if (op[0] === "A") {
                d += "A " + fmt(op[1] * s) + " " + fmt(op[1] * s) + " 0 " + op[2] + " " + op[3]
                    + " " + fmt(x0 + op[4] * s) + " " + fmt(y0 + op[5] * s) + " ";
            } else {
                d += op[0] + " " + fmt(x0 + op[1] * s) + " " + fmt(y0 + op[2] * s) + " ";
            }
        }
        return d + "Z";
    }

    /*! A circle as two half arcs. */
    function circlePath(cx, cy, r) {
        return "M " + fmt(cx - r) + " " + fmt(cy) + " A " + fmt(r) + " " + fmt(r) + " 0 1 0 "
            + fmt(cx + r) + " " + fmt(cy) + " A " + fmt(r) + " " + fmt(r) + " 0 1 0 "
            + fmt(cx - r) + " " + fmt(cy) + " Z";
    }

    /*! An ellipse with radii rx and ry. */
    function ellipsePath(cx, cy, rx, ry) {
        return "M " + fmt(cx - rx) + " " + fmt(cy) + " A " + fmt(rx) + " " + fmt(ry) + " 0 1 0 "
            + fmt(cx + rx) + " " + fmt(cy) + " A " + fmt(rx) + " " + fmt(ry) + " 0 1 0 "
            + fmt(cx - rx) + " " + fmt(cy) + " Z";
    }

    /*! The arc of a circle from angle a0 to a1 (degrees, screen angles, increasing). */
    function arcPath(cx, cy, r, a0, a1) {
        const p = a => [cx + r * Math.cos(a * Math.PI / 180), cy + r * Math.sin(a * Math.PI / 180)];
        const s = p(a0);
        const e = p(a1);
        const large = ((a1 - a0) % 360 + 360) % 360 > 180 ? 1 : 0;
        return "M " + fmt(s[0]) + " " + fmt(s[1]) + " A " + fmt(r) + " " + fmt(r) + " 0 " + large + " 1 "
            + fmt(e[0]) + " " + fmt(e[1]);
    }

    /*! A triangular sun ray: base at radius r0 (half width `half`), tip at r1. */
    function rayPath(cx, cy, r0, r1, half, deg) {
        const t = deg * Math.PI / 180;
        const ux = Math.cos(t);
        const uy = Math.sin(t);
        const bx = cx + r0 * ux;
        const by = cy + r0 * uy;
        return "M " + fmt(bx - half * uy) + " " + fmt(by + half * ux) + " L " + fmt(cx + r1 * ux) + " "
            + fmt(cy + r1 * uy) + " L " + fmt(bx + half * uy) + " " + fmt(by - half * ux) + " Z";
    }

    /*! A raindrop: a point above a circle of radius r, its sides tangent to the circle. */
    function dropPath(cx, cy, r) {
        const k = 2.1;
        const a = Math.acos(1 / k);
        const pr = [cx + r * Math.cos(-Math.PI / 2 + a), cy + r * Math.sin(-Math.PI / 2 + a)];
        const pl = [cx + r * Math.cos(-Math.PI / 2 - a), cy + r * Math.sin(-Math.PI / 2 - a)];
        return "M " + fmt(cx) + " " + fmt(cy - k * r) + " L " + fmt(pr[0]) + " " + fmt(pr[1])
            + " A " + fmt(r) + " " + fmt(r) + " 0 1 1 " + fmt(pl[0]) + " " + fmt(pl[1]) + " Z";
    }

    /*! A rounded bar, height h, corners of radius h / 2. */
    function barPath(x, y, w, h) {
        const r = h / 2;
        return "M " + fmt(x + r) + " " + fmt(y) + " H " + fmt(x + w - r)
            + " A " + fmt(r) + " " + fmt(r) + " 0 0 1 " + fmt(x + w - r) + " " + fmt(y + h)
            + " H " + fmt(x + r) + " A " + fmt(r) + " " + fmt(r) + " 0 0 1 " + fmt(x + r) + " " + fmt(y) + " Z";
    }

    /*! A six-armed crystal: six spokes from the centre, a fork on each. */
    function crystalPath(cx, cy, R) {
        let d = "";
        for (let i = 0; i < 6; ++i) {
            const t = (-90 + 60 * i) * Math.PI / 180;
            const ux = Math.cos(t);
            const uy = Math.sin(t);
            d += "M " + fmt(cx) + " " + fmt(cy) + " L " + fmt(cx + R * ux) + " " + fmt(cy + R * uy) + " ";
            const bx = cx + 0.55 * R * ux;
            const by = cy + 0.55 * R * uy;
            for (const sgn of [1, -1]) {
                const f = t + sgn * 0.87;
                d += "M " + fmt(bx) + " " + fmt(by) + " L " + fmt(bx + 0.4 * R * Math.cos(f)) + " "
                    + fmt(by + 0.4 * R * Math.sin(f)) + " ";
            }
        }
        return d;
    }

    /*! A four-pointed star with concave sides. */
    function starPath(cx, cy, r) {
        return "M " + fmt(cx) + " " + fmt(cy - r) + " Q " + fmt(cx) + " " + fmt(cy) + " " + fmt(cx + r) + " " + fmt(cy)
            + " Q " + fmt(cx) + " " + fmt(cy) + " " + fmt(cx) + " " + fmt(cy + r)
            + " Q " + fmt(cx) + " " + fmt(cy) + " " + fmt(cx - r) + " " + fmt(cy)
            + " Q " + fmt(cx) + " " + fmt(cy) + " " + fmt(cx) + " " + fmt(cy - r) + " Z";
    }

    // ---- parts ----------------------------------------------------------------------

    /*! The sun: eight triangular rays, each with its own gradient, and a golden disc. */
    component Sun: Shape {
        id: sun

        required property real cx
        required property real cy
        required property real r

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        Ray { deg: 0; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 45; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 90; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 135; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 180; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 225; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 270; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }
        Ray { deg: 315; cx: sun.cx; cy: sun.cy; r0: sun.r + 2.8; r1: sun.r + 9.6 }

        ShapePath {
            strokeColor: root.steel
            strokeWidth: root.hair
            fillGradient: RadialGradient {
                centerX: sun.cx - sun.r * 0.3
                centerY: sun.cy - sun.r * 0.3
                focalX: sun.cx - sun.r * 0.3
                focalY: sun.cy - sun.r * 0.3
                centerRadius: sun.r * 1.7
                GradientStop { position: 0; color: "#fff7b0" }
                GradientStop { position: 0.55; color: "#ffcc33" }
                GradientStop { position: 1; color: "#f08c00" }
            }
            PathSvg { path: root.circlePath(sun.cx, sun.cy, sun.r) }
        }
        ShapePath {
            strokeColor: root.gloss
            strokeWidth: 1.9
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.arcPath(sun.cx, sun.cy, sun.r - 2.6, 200, 250) }
        }
    }

    /*! One triangular ray, its gradient running from the base to the tip. */
    component Ray: ShapePath {
        required property real deg
        required property real cx
        required property real cy
        required property real r0
        required property real r1

        readonly property real rad: deg * Math.PI / 180
        readonly property real bx: cx + r0 * Math.cos(rad)
        readonly property real by: cy + r0 * Math.sin(rad)
        readonly property real tx: cx + r1 * Math.cos(rad)
        readonly property real ty: cy + r1 * Math.sin(rad)

        strokeColor: "#b8700f"
        strokeWidth: root.hair
        fillGradient: LinearGradient {
            x1: bx
            y1: by
            x2: tx
            y2: ty
            GradientStop { position: 0; color: "#ffe47a" }
            GradientStop { position: 1; color: "#ff9a1f" }
        }
        PathSvg { path: root.rayPath(cx, cy, r0, r1, 2.4, deg) }
    }

    /*! The crescent moon with a gloss arc along its outer edge. */
    component Moon: Shape {
        id: moon

        required property string path
        required property real cx
        required property real cy
        required property real r

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.steel
            strokeWidth: root.hair
            fillGradient: RadialGradient {
                centerX: moon.cx - moon.r * 0.5
                centerY: moon.cy - moon.r * 0.5
                focalX: moon.cx - moon.r * 0.5
                focalY: moon.cy - moon.r * 0.5
                centerRadius: moon.r * 2.1
                GradientStop { position: 0; color: "#ffffff" }
                GradientStop { position: 0.5; color: "#fffbe0" }
                GradientStop { position: 0.8; color: "#fff0a0" }
                GradientStop { position: 1; color: "#e3c867" }
            }
            PathSvg { path: moon.path }
        }
        ShapePath {
            strokeColor: root.gloss
            strokeWidth: 1.6
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.arcPath(moon.cx, moon.cy, moon.r - 2.2, 200, 250) }
        }
    }

    /*! A Luna cloud at (x0, y0), scaled by s, with its gloss on the upper bump. */
    component Cloud: Shape {
        id: cloud

        required property real x0
        required property real y0
        required property real s
        required property color hi
        required property color mid
        required property color lo
        required property color edge

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: cloud.edge
            strokeWidth: root.hair
            fillGradient: LinearGradient {
                x1: cloud.x0 + 2 * cloud.s
                y1: cloud.y0 + 4 * cloud.s
                x2: cloud.x0 + 2 * cloud.s
                y2: cloud.y0 + 23 * cloud.s
                GradientStop { position: 0; color: cloud.hi }
                GradientStop { position: 0.45; color: cloud.mid }
                GradientStop { position: 1; color: cloud.lo }
            }
            PathSvg { path: root.cloudPath(cloud.x0, cloud.y0, cloud.s) }
        }
        ShapePath {
            strokeColor: root.gloss
            strokeWidth: 2.2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.arcPath(cloud.x0 + 17 * cloud.s, cloud.y0 + 12 * cloud.s, 5.8 * cloud.s, 200, 255) }
        }
    }

    /*! A glossy raindrop: blue from the tip down, a white shine on its upper left. */
    component Drop: Shape {
        id: drop

        required property real cx
        required property real cy
        required property real r
        required property color hi
        required property color lo

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.navy
            strokeWidth: root.hair
            fillGradient: LinearGradient {
                x1: drop.cx
                y1: drop.cy - 2.1 * drop.r
                x2: drop.cx
                y2: drop.cy + drop.r
                GradientStop { position: 0; color: drop.hi }
                GradientStop { position: 1; color: drop.lo }
            }
            PathSvg { path: root.dropPath(drop.cx, drop.cy, drop.r) }
        }
        ShapePath {
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: drop.cx - drop.r * 0.35
                centerY: drop.cy + drop.r * 0.15
                centerRadius: drop.r * 0.55
                GradientStop { position: 0; color: root.gloss }
                GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
            }
            PathSvg { path: root.ellipsePath(drop.cx - drop.r * 0.35, drop.cy + drop.r * 0.15, drop.r * 0.3, drop.r * 0.5) }
        }
    }

    /*! A six-armed crystal: a blue outline under a white core. */
    component Crystal: Shape {
        id: crystal

        required property real cx
        required property real cy
        required property real r

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#2a62c9"
            strokeWidth: 2.7
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.crystalPath(crystal.cx, crystal.cy, crystal.r) }
        }
        ShapePath {
            strokeColor: "#ffffff"
            strokeWidth: 1.2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.crystalPath(crystal.cx, crystal.cy, crystal.r) }
        }
    }

    /*! One fog bar: a grey-blue, translucent, rounded band. */
    component Bar: ShapePath {
        required property real bx
        required property real by
        required property real bw
        required property real alpha

        strokeColor: root.steel
        strokeWidth: root.hair
        fillGradient: LinearGradient {
            x1: 0
            y1: by
            x2: 0
            y2: by + 5
            GradientStop { position: 0; color: Qt.rgba(0.91, 0.94, 0.97, alpha) }
            GradientStop { position: 1; color: Qt.rgba(0.62, 0.7, 0.8, alpha) }
        }
        PathSvg { path: root.barPath(bx, by, bw, 5) }
    }

    // ---- the scene -------------------------------------------------------------------

    Item {
        id: art

        width: root.design
        height: root.design
        transformOrigin: Item.TopLeft
        scale: root.unit
        x: (root.width - root.side) / 2
        y: (root.height - root.side) / 2

        /*! A faint blue-white halo behind the art, only on a dark surface. */
        Shape {
            visible: root.halo
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                fillGradient: RadialGradient {
                    centerX: 24
                    centerY: 24
                    centerRadius: 24
                    GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.34) }
                    GradientStop { position: 0.6; color: Qt.rgba(1, 1, 1, 0.14) }
                    GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
                }
                PathSvg { path: root.circlePath(24, 24, 24) }
            }
        }

        /*! The static shadow under the object. */
        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                fillColor: Qt.rgba(0.24, 0.29, 0.38, 0.2)
                PathSvg { path: root.ellipsePath(24, 45.5, 12.5, 2.0) }
            }
            ShapePath {
                strokeColor: "transparent"
                fillColor: Qt.rgba(0.24, 0.29, 0.38, 0.12)
                PathSvg { path: root.ellipsePath(24, 45.5, 8, 1.2) }
            }
        }

        /*! The night sky: a hint of deep blue behind the moon. */
        Shape {
            visible: root.moony
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                fillGradient: RadialGradient {
                    centerX: 24
                    centerY: 24
                    centerRadius: 24
                    GradientStop { position: 0; color: Qt.rgba(0.11, 0.23, 0.42, 0.42) }
                    GradientStop { position: 1; color: Qt.rgba(0.11, 0.23, 0.42, 0) }
                }
                PathSvg { path: root.circlePath(24, 24, 24) }
            }
        }

        Sun {
            visible: root.sunny
            cx: root.sunC
            cy: root.sunC
            r: root.sunR
        }

        Moon {
            visible: root.moony
            path: root.partly ? root.moonPartly : root.moonDay
            cx: root.partly ? 16 : 24
            cy: root.partly ? 15 : 24
            r: root.partly ? 8.5 : 13
        }

        /*! Small stars: three under a clear sky, one beside a cloudy moon. */
        Shape {
            visible: root.moony
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: root.steel
                strokeWidth: root.hair
                fillColor: "#ffffff"
                PathSvg { path: root.starPath(39, 10, 2.6) }
            }
            ShapePath {
                strokeColor: root.steel
                strokeWidth: root.hair
                fillColor: "#ffffff"
                PathSvg { path: root.starPath(8.5, 9.5, 3.2) }
            }
            ShapePath {
                strokeColor: root.steel
                strokeWidth: root.hair
                fillColor: "#ffffff"
                PathSvg { path: root.starPath(40, 37, 1.9) }
            }
        }

        Cloud {
            visible: root.cloudy
            x0: 12
            y0: 2
            s: 0.8
            hi: "#f2f7fd"
            mid: "#c4d5ea"
            lo: "#8aa2c2"
            edge: root.steel
        }

        Cloud {
            visible: root.hasCloud
            x0: root.cloudX
            y0: root.cloudY
            s: 1
            hi: "#ffffff"
            mid: "#cfe3f7"
            lo: "#9fbfe3"
            edge: root.steel
        }

        Cloud {
            visible: root.condition === "thunderstorm"
            x0: root.cloudX
            y0: root.cloudY
            s: 1
            hi: "#dfe8f4"
            mid: "#8ea4c4"
            lo: "#5a6e8c"
            edge: root.navy
        }

        Drop {
            visible: root.condition === "drizzle"
            cx: 15; cy: 33; r: 2.2
            hi: "#c9e4ff"; lo: "#6f9fe0"
        }
        Drop {
            visible: root.condition === "drizzle"
            cx: 24; cy: 37; r: 2.2
            hi: "#c9e4ff"; lo: "#6f9fe0"
        }
        Drop {
            visible: root.condition === "drizzle"
            cx: 33; cy: 33; r: 2.2
            hi: "#c9e4ff"; lo: "#6f9fe0"
        }

        Drop {
            visible: root.condition === "rain"
            cx: 14; cy: 34; r: 3
            hi: "#7fb2ee"; lo: "#2a62c9"
        }
        Drop {
            visible: root.condition === "rain"
            cx: 24; cy: 38; r: 3
            hi: "#7fb2ee"; lo: "#2a62c9"
        }
        Drop {
            visible: root.condition === "rain"
            cx: 34; cy: 34; r: 3
            hi: "#7fb2ee"; lo: "#2a62c9"
        }

        Drop {
            visible: root.condition === "heavy_rain"
            cx: 10; cy: 33; r: 2.6
            hi: "#7fb2ee"; lo: "#2a62c9"
        }
        Drop {
            visible: root.condition === "heavy_rain"
            cx: 19; cy: 38; r: 2.8
            hi: "#7fb2ee"; lo: "#2a62c9"
        }
        Drop {
            visible: root.condition === "heavy_rain"
            cx: 29; cy: 33; r: 2.8
            hi: "#7fb2ee"; lo: "#2a62c9"
        }
        Drop {
            visible: root.condition === "heavy_rain"
            cx: 38; cy: 38; r: 2.6
            hi: "#7fb2ee"; lo: "#2a62c9"
        }

        /*! Lightning: a yellow bolt with an orange outline. */
        Shape {
            visible: root.condition === "thunderstorm"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "#e8790a"
                strokeWidth: root.hair * 1.4
                joinStyle: ShapePath.MiterJoin
                fillGradient: LinearGradient {
                    x1: 0
                    y1: 25
                    x2: 0
                    y2: 46
                    GradientStop { position: 0; color: "#fff7a8" }
                    GradientStop { position: 0.5; color: "#ffd83d" }
                    GradientStop { position: 1; color: "#ffae00" }
                }
                PathSvg { path: "M 27 25 L 18.5 35.5 L 24.5 35.5 L 20.5 46 L 31 33 L 25 33 L 29.5 25 Z" }
            }
        }

        /*! Snow: three six-armed crystals under the cloud. */
        Crystal {
            visible: root.condition === "snow"
            cx: 13; cy: 35; r: 5
        }
        Crystal {
            visible: root.condition === "snow"
            cx: 24; cy: 39.5; r: 5
        }
        Crystal {
            visible: root.condition === "snow"
            cx: 35; cy: 35; r: 5
        }

        /*! Fog: four grey-blue translucent bars; mist: three lighter ones. */
        Shape {
            visible: root.condition === "fog"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            Bar { bx: 14; by: 9; bw: 26; alpha: 0.9 }
            Bar { bx: 5; by: 18.5; bw: 38; alpha: 0.9 }
            Bar { bx: 10; by: 28; bw: 28; alpha: 0.9 }
            Bar { bx: 4; by: 37.5; bw: 40; alpha: 0.9 }
        }
        Shape {
            visible: root.condition === "mist" || root.condition === "atmospheric"
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            Bar { bx: 10; by: 12; bw: 28; alpha: 0.6 }
            Bar { bx: 5; by: 22.5; bw: 38; alpha: 0.6 }
            Bar { bx: 13; by: 33; bw: 22; alpha: 0.6 }
        }

        /*! Unknown: the Windows XP help balloon with a question mark. */
        Shape {
            visible: root.help
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: root.navy
                strokeWidth: root.hair
                fillGradient: LinearGradient {
                    x1: 0
                    y1: 8
                    x2: 0
                    y2: 34
                    GradientStop { position: 0; color: "#fffef4" }
                    GradientStop { position: 0.5; color: "#fff3b8" }
                    GradientStop { position: 1; color: "#f5cf55" }
                }
                PathSvg { path: "M 15 8 H 33 A 9 9 0 0 1 42 17 V 25 A 9 9 0 0 1 33 34 H 22 L 13 42 L 15 34 A 9 9 0 0 1 6 25 V 17 A 9 9 0 0 1 15 8 Z" }
            }
            ShapePath {
                strokeColor: root.gloss
                strokeWidth: 1.6
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathSvg { path: root.arcPath(15, 17, 6.5, 200, 250) }
            }
            ShapePath {
                strokeColor: root.navy
                strokeWidth: 5.4
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathSvg { path: "M 17.5 17.5 C 17.5 11 30.5 11 30.5 18 C 30.5 22 24.5 22 24.5 26.5 L 24.5 28.5" }
            }
            ShapePath {
                strokeColor: "#245edb"
                strokeWidth: 3.2
                fillColor: "transparent"
                capStyle: ShapePath.RoundCap
                PathSvg { path: "M 17.5 17.5 C 17.5 11 30.5 11 30.5 18 C 30.5 22 24.5 22 24.5 26.5 L 24.5 28.5" }
            }
            ShapePath {
                strokeColor: "transparent"
                fillColor: "#245edb"
                PathSvg { path: root.circlePath(24.5, 33.5, 2.3) }
            }
        }
    }
}
