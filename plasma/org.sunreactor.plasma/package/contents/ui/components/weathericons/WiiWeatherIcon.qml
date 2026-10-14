/*
 * Wii weather icon, in the Forecast Channel's own look (2006-2009): puffy white
 * clouds with a light blue belly and a faint edge, a warm golden sun with a soft
 * glow ring and tapered rays, glossy Wii-blue rain, a golden bolt with a white
 * core, white snow with a soft blue edge, translucent fog bands, and at night a
 * pale moon with twinkling stars. Vector only: Shapes with gradients, no images,
 * no blur. The soft shadow under a cloud is a radial gradient squashed flat.
 *
 * Drawn on a 64-unit design grid and scaled to `size`, so it stays sharp at any
 * size and device pixel ratio. Every colour is the channel's own: the clouds carry
 * a blue belly and a faint edge, so they read on the near-white Wii face and on a
 * dark panel alike. surface and ink come with the registry contract and are unused.
 *
 *   WiiWeatherIcon { theme: view.theme; condition: "rain"; night: false; size: 22 }
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

Item {
    id: root

    /*! Daemon condition token; "" is the unknown icon. */
    required property string condition
    required property bool night
    required property var theme
    /*! Logical px, square. */
    property real size: 22
    property color surface: theme.bg
    property color ink: theme.fg

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    /*! What the condition draws, in design units (see planFor). */
    readonly property var plan: planFor(condition, night)

    Item {
        id: art

        width: 64
        height: 64
        transformOrigin: Item.TopLeft
        scale: root.size / 64

        // Back to front: the sky body, stars, clouds, then what falls or sits in front.
        Repeater {
            model: root.plan.suns

            delegate: Sun {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.moons

            delegate: Moon {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.stars

            delegate: Star {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.clouds

            delegate: Cloud {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.drops

            delegate: Drop {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.flakes

            delegate: Flake {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.bolts

            delegate: Bolt {}
        }
        Repeater {
            model: root.plan.bands

            delegate: Band {
                required property var modelData

                spec: modelData
            }
        }
        Repeater {
            model: root.plan.marks

            delegate: Mark {}
        }
    }

    // --- what each condition draws (design units, 64 x 64) -----------------------

    function planFor(c, night) {
        const p = {
            suns: [], moons: [], stars: [], clouds: [], drops: [],
            flakes: [], bands: [], bolts: [], marks: []
        };
        const sky = function (x, y, r) {
            if (night) {
                p.moons.push({ x: x, y: y, r: r });
            } else {
                p.suns.push({ x: x, y: y, r: r });
            }
        };
        switch (c) {
        case "clear":
            sky(32, 32, 14);
            if (night) {
                p.stars = [{ x: 13, y: 14, r: 4 }, { x: 50, y: 15, r: 2.6 }, { x: 49, y: 49, r: 3.4 }];
            }
            break;
        case "partly_cloudy":
            sky(24, 23, 11);
            if (night) {
                p.stars = [{ x: 50, y: 12, r: 3.2 }];
            }
            p.clouds = [{ x: 12, y: 19, s: 0.95, tone: "front" }];
            break;
        case "cloudy":
            p.clouds = [{ x: 30, y: 6, s: 0.7, tone: "back" }, { x: 5, y: 16, s: 1, tone: "front" }];
            break;
        case "drizzle":
            p.clouds = [{ x: 8, y: 10, s: 0.95, tone: "front" }];
            p.drops = [
                { x: 19, top: 40, w: 4.4, h: 9.5 },
                { x: 32, top: 46, w: 4.4, h: 9.5 },
                { x: 45, top: 40, w: 4.4, h: 9.5 }
            ];
            break;
        case "rain":
            p.clouds = [{ x: 8, y: 8, s: 0.95, tone: "front" }];
            p.drops = [
                { x: 18, top: 41, w: 5.2, h: 12 },
                { x: 32, top: 46, w: 5.2, h: 12 },
                { x: 46, top: 41, w: 5.2, h: 12 }
            ];
            break;
        case "heavy_rain":
            p.clouds = [{ x: 4, y: 6, s: 1, tone: "storm" }];
            p.drops = [
                { x: 14, top: 42, w: 5, h: 12 },
                { x: 26, top: 48, w: 5, h: 12 },
                { x: 38, top: 42, w: 5, h: 12 },
                { x: 50, top: 48, w: 5, h: 12 }
            ];
            break;
        case "thunderstorm":
            p.clouds = [{ x: 5, y: 5, s: 1, tone: "storm" }];
            p.bolts = [{}];
            p.drops = [
                { x: 13, top: 44, w: 4.4, h: 9.5 },
                { x: 52, top: 42, w: 4.4, h: 9.5 }
            ];
            break;
        case "snow":
            p.clouds = [{ x: 8, y: 8, s: 0.95, tone: "front" }];
            p.flakes = [
                { x: 17, y: 49, r: 6 },
                { x: 33, y: 55, r: 5.2 },
                { x: 47, y: 48, r: 5.6 }
            ];
            break;
        case "mist":
            p.bands = [
                { x1: 8, x2: 56, y: 24, w: 3.8, a: 1 },
                { x1: 16, x2: 48, y: 33, w: 3.8, a: 1 },
                { x1: 6, x2: 44, y: 42, w: 3.8, a: 1 },
                { x1: 18, x2: 56, y: 51, w: 3.8, a: 1 }
            ];
            break;
        case "atmospheric":
            p.suns.push({ x: 32, y: 26, r: 12, faint: true });
            p.bands = [
                { x1: 10, x2: 54, y: 42, w: 4.4, a: 1 },
                { x1: 18, x2: 46, y: 51, w: 4.4, a: 1 }
            ];
            break;
        case "fog":
            p.clouds = [{ x: 14, y: 4, s: 0.9, tone: "back" }];
            p.bands = [
                { x1: 6, x2: 50, y: 36, w: 4.8, a: 1 },
                { x1: 14, x2: 58, y: 45, w: 4.8, a: 1 },
                { x1: 8, x2: 46, y: 54, w: 4.8, a: 1 }
            ];
            break;
        default:
            p.clouds = [{ x: 6, y: 10, s: 1, tone: "dim" }];
            p.marks = [{}];
            break;
        }
        return p;
    }

    // --- colour -------------------------------------------------------------------

    /*! Cloud tones: body gradient stops (top, mid, belly, base), edge, shadow. */
    readonly property var tones: ({
        front: { top: "#f6fbff", mid: "#fbfdff", belly: "#e2eff9", base: "#d2e5f5", edge: "#c3d6e6" },
        back: { top: "#f0f7fd", mid: "#e4eff9", belly: "#c4dcf0", base: "#afcee8", edge: "#b8d0e4" },
        storm: { top: "#eaf2f9", mid: "#d2e2ef", belly: "#aac5dc", base: "#8fb1cc", edge: "#8ea8bf" },
        dim: { top: "#ffffff", mid: "#eef3f7", belly: "#d7e1ea", base: "#c4d3e0", edge: "#b4c6d5" }
    })

    // --- geometry: SVG path strings in design units ---------------------------------

    readonly property real degRad: Math.PI / 180

    function num(v) {
        return Number(v).toFixed(2);
    }

    function pt(x, y) {
        return num(x) + " " + num(y);
    }

    /*! A full circle, clockwise on screen (every closed subpath here winds the same way,
        so overlapping subpaths fill as one union under the nonzero rule). */
    function circle(cx, cy, r) {
        return "M" + pt(cx - r, cy) + "A" + num(r) + " " + num(r) + " 0 1 1 " + pt(cx + r, cy)
            + "A" + num(r) + " " + num(r) + " 0 1 1 " + pt(cx - r, cy) + "Z";
    }

    /*! The arc of a circle from angle a1 to a2 in degrees, clockwise on screen. */
    function arc(cx, cy, r, a1, a2) {
        const t1 = a1 * degRad;
        const t2 = a2 * degRad;
        const large = ((a2 - a1) % 360 + 360) % 360 > 180 ? 1 : 0;
        return "M" + pt(cx + r * Math.cos(t1), cy + r * Math.sin(t1))
            + "A" + num(r) + " " + num(r) + " 0 " + large + " 1 "
            + pt(cx + r * Math.cos(t2), cy + r * Math.sin(t2));
    }

    function line(x1, y1, x2, y2) {
        return "M" + pt(x1, y1) + "L" + pt(x2, y2);
    }

    /*! The cloud: three bumps on a flat base with round ends, in local units scaled by c.s. */
    function cloudPath(c) {
        const s = c.s;
        const bumps = [[22, 13, 11], [12, 19, 9], [33, 18, 9], [10, 21, 9], [34, 21, 9]];
        let d = "";
        for (let i = 0; i < bumps.length; ++i) {
            d += circle(c.x + bumps[i][0] * s, c.y + bumps[i][1] * s, bumps[i][2] * s);
        }
        d += "M" + pt(c.x + 10 * s, c.y + 12 * s)
            + "H" + num(c.x + 34 * s) + "V" + num(c.y + 30 * s)
            + "H" + num(c.x + 10 * s) + "Z";
        return d;
    }

    /*! The rim light: arcs just inside the top of the bumps. */
    function cloudRim(c) {
        const s = c.s;
        return arc(c.x + 22 * s, c.y + 13 * s, 9.4 * s, 200, 285)
            + arc(c.x + 12 * s, c.y + 19 * s, 7.4 * s, 200, 262)
            + arc(c.x + 33 * s, c.y + 18 * s, 7.4 * s, 262, 330);
    }

    /*! Eight tapered rays, each a kite from the disc's rim out to 1.98 r. */
    function sunRays(cx, cy, r) {
        let d = "";
        for (let k = 0; k < 8; ++k) {
            const t = (k * 45 - 90) * degRad;
            const ux = Math.cos(t);
            const uy = Math.sin(t);
            const nx = -uy;
            const ny = ux;
            const rb = r * 1.42;
            const rt = r * 1.98;
            const hb = r * 0.2;
            const ht = r * 0.07;
            d += "M" + pt(cx + rb * ux + hb * nx, cy + rb * uy + hb * ny)
                + "L" + pt(cx + rt * ux + ht * nx, cy + rt * uy + ht * ny)
                + "L" + pt(cx + rt * ux - ht * nx, cy + rt * uy - ht * ny)
                + "L" + pt(cx + rb * ux - hb * nx, cy + rb * uy - hb * ny) + "Z";
        }
        return d;
    }

    /*! A raindrop: a point at the top, a round bottom of radius w / 2. */
    function dropPath(g) {
        const r = g.w / 2;
        const yc = g.top + g.h - r;
        return "M" + pt(g.x, g.top)
            + "C" + pt(g.x + 0.55 * r, g.top + 0.42 * g.h) + " " + pt(g.x + r, yc - 0.5 * r) + " " + pt(g.x + r, yc)
            + "A" + num(r) + " " + num(r) + " 0 0 1 " + pt(g.x - r, yc)
            + "C" + pt(g.x - r, yc - 0.5 * r) + " " + pt(g.x - 0.55 * r, g.top + 0.42 * g.h) + " " + pt(g.x, g.top)
            + "Z";
    }

    /*! The gloss streak on the left of a raindrop. */
    function dropStreak(g) {
        const r = g.w / 2;
        const yc = g.top + g.h - r;
        return line(g.x - 0.5 * r, yc - 0.15 * r, g.x - 0.45 * r, yc - 0.75 * r);
    }

    /*! Six arms, each with two short branches at 55 percent of its length. */
    function flakePath(cx, cy, R) {
        let d = "";
        for (let k = 0; k < 6; ++k) {
            const t = (k * 60 - 90) * degRad;
            const ux = Math.cos(t);
            const uy = Math.sin(t);
            d += line(cx, cy, cx + R * ux, cy + R * uy);
            const bx = cx + 0.55 * R * ux;
            const by = cy + 0.55 * R * uy;
            for (let side = -1; side <= 1; side += 2) {
                const tb = t + side * 45 * degRad;
                d += line(bx, by, bx + 0.36 * R * Math.cos(tb), by + 0.36 * R * Math.sin(tb));
            }
        }
        return d;
    }

    /*! A four-point twinkle: concave sides, points on the axes. */
    function starPath(cx, cy, r) {
        const k = r * 0.2;
        return "M" + pt(cx, cy - r)
            + "Q" + pt(cx + k, cy - k) + " " + pt(cx + r, cy)
            + "Q" + pt(cx + k, cy + k) + " " + pt(cx, cy + r)
            + "Q" + pt(cx - k, cy + k) + " " + pt(cx - r, cy)
            + "Q" + pt(cx - k, cy - k) + " " + pt(cx, cy - r) + "Z";
    }

    /*! The moon's crescent: the outer disc minus an inner disc, built from its two
        intersection points so no fill rule is needed. */
    function crescentPath(cx, cy, R, ix, iy, ir) {
        const dx = ix - cx;
        const dy = iy - cy;
        const d = Math.hypot(dx, dy);
        const a = (R * R - ir * ir + d * d) / (2 * d);
        const h = Math.sqrt(Math.max(0, R * R - a * a));
        const px = cx + a * dx / d;
        const py = cy + a * dy / d;
        const p1 = [px - h * dy / d, py + h * dx / d];
        const p2 = [px + h * dy / d, py - h * dx / d];
        const outerVia = [cx - R * dx / d, cy - R * dy / d];
        const innerVia = [ix - ir * dx / d, iy - ir * dy / d];
        return "M" + pt(p1[0], p1[1])
            + arcThrough(cx, cy, R, p1, p2, outerVia)
            + arcThrough(ix, iy, ir, p2, p1, innerVia) + "Z";
    }

    /*! An SVG arc command from `from` to `to` on a circle, passing through `via`. */
    function arcThrough(cx, cy, r, from, to, via) {
        const TAU = 2 * Math.PI;
        const ang = function (p) {
            return Math.atan2(p[1] - cy, p[0] - cx);
        };
        const a0 = ang(from);
        const span = ((ang(to) - a0) % TAU + TAU) % TAU;
        const toVia = ((ang(via) - a0) % TAU + TAU) % TAU;
        const sweep = toVia < span ? 1 : 0;
        const total = sweep ? span : TAU - span;
        const large = total > Math.PI ? 1 : 0;
        return "A" + num(r) + " " + num(r) + " 0 " + large + " " + sweep + " " + pt(to[0], to[1]);
    }

    /*! The bolt, a zig-zag of seven points; f scales it about its centre (0.5 for the core). */
    function boltPath(f) {
        const pts = [[38, 30], [48, 30], [38, 42], [45, 42], [30, 60], [34, 47], [27, 47]];
        const cx = 38;
        const cy = 42;
        let d = "";
        for (let i = 0; i < pts.length; ++i) {
            const x = cx + f * (pts[i][0] - cx);
            const y = cy + f * (pts[i][1] - cy);
            d += (i === 0 ? "M" : "L") + pt(x, y);
        }
        return d + "Z";
    }

    /*! The question mark on a cloud: a hook and a dot. */
    function questionPath() {
        return "M" + pt(23, 23) + "Q" + pt(23, 17) + " " + pt(28, 17)
            + "Q" + pt(33, 17) + " " + pt(33, 22.5)
            + "Q" + pt(33, 26.5) + " " + pt(28, 29)
            + "Q" + pt(28, 31) + " " + pt(28, 33);
    }

    // --- elements (each an Item or Shape filling the art) ----------------------------

    /*! A cloud: the soft shadow under it, the faint edge, the body and the rim light. */
    component Cloud: Item {
        id: cloud

        required property var spec
        readonly property var tone: root.tones[spec.tone || "front"]
        readonly property real s: spec.s

        anchors.fill: parent

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer
            // A blurred ellipse: a radial gradient squashed flat under the cloud.
            transform: Scale {
                origin.x: cloud.spec.x + 22 * cloud.s
                origin.y: cloud.spec.y + 31 * cloud.s
                yScale: 0.2
            }

            ShapePath {
                strokeColor: "transparent"
                fillRule: ShapePath.WindingFill
                fillGradient: RadialGradient {
                    centerX: cloud.spec.x + 22 * cloud.s
                    centerY: cloud.spec.y + 31 * cloud.s
                    centerRadius: 21 * cloud.s

                    GradientStop { position: 0; color: "#3d2a6fa3" }
                    GradientStop { position: 0.5; color: "#1f2a6fa3" }
                    GradientStop { position: 1; color: "#002a6fa3" }
                }
                PathSvg {
                    path: root.circle(cloud.spec.x + 22 * cloud.s, cloud.spec.y + 31 * cloud.s, 21 * cloud.s)
                }
            }
        }

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                // The faint edge: the body's outline, half its width outside the body.
                fillColor: cloud.tone.edge
                fillRule: ShapePath.WindingFill
                strokeColor: cloud.tone.edge
                strokeWidth: 1.2 * cloud.s
                joinStyle: ShapePath.RoundJoin
                PathSvg { path: root.cloudPath(cloud.spec) }
            }
            ShapePath {
                // The body: white at the top, a blue belly below.
                fillRule: ShapePath.WindingFill
                strokeColor: "transparent"
                fillGradient: LinearGradient {
                    x1: cloud.spec.x
                    y1: cloud.spec.y + 2 * cloud.s
                    x2: cloud.spec.x
                    y2: cloud.spec.y + 30 * cloud.s

                    GradientStop { position: 0; color: cloud.tone.top }
                    GradientStop { position: 0.45; color: cloud.tone.mid }
                    GradientStop { position: 0.8; color: cloud.tone.belly }
                    GradientStop { position: 1; color: cloud.tone.base }
                }
                PathSvg { path: root.cloudPath(cloud.spec) }
            }
            ShapePath {
                // The rim light along the top of the bumps.
                fillColor: "transparent"
                strokeColor: "#e6ffffff"
                strokeWidth: 1.3 * cloud.s
                capStyle: ShapePath.RoundCap
                PathSvg { path: root.cloudRim(cloud.spec) }
            }
        }
    }

    /*! The sun: a glow ring, eight rays, a disc with a golden radial gradient. */
    component Sun: Shape {
        id: sun

        required property var spec

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        opacity: spec.faint ? 0.5 : 1

        ShapePath {
            // The glow: translucent, a ring around the disc.
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: sun.spec.x
                centerY: sun.spec.y
                centerRadius: sun.spec.r * 2.3

                GradientStop { position: 0; color: "#00ffe27a" }
                GradientStop { position: 0.43; color: "#00ffe27a" }
                GradientStop { position: 0.62; color: "#59ffc82e" }
                GradientStop { position: 1; color: "#00ffc82e" }
            }
            PathSvg { path: root.circle(sun.spec.x, sun.spec.y, sun.spec.r * 2.3) }
        }
        ShapePath {
            fillColor: "#ffc82e"
            strokeColor: "#ffc82e"
            strokeWidth: sun.spec.r * 0.16
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.sunRays(sun.spec.x, sun.spec.y, sun.spec.r) }
        }
        ShapePath {
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: sun.spec.x
                centerY: sun.spec.y
                centerRadius: sun.spec.r

                GradientStop { position: 0; color: "#fff6a8" }
                GradientStop { position: 0.55; color: "#ffc82e" }
                GradientStop { position: 1; color: "#ff9f1c" }
            }
            PathSvg { path: root.circle(sun.spec.x, sun.spec.y, sun.spec.r) }
        }
        ShapePath {
            // The gloss.
            fillColor: "#8cffffff"
            strokeColor: "transparent"
            PathSvg {
                path: root.circle(sun.spec.x - 0.34 * sun.spec.r, sun.spec.y - 0.34 * sun.spec.r,
                                  0.26 * sun.spec.r)
            }
        }
    }

    /*! The moon: a pale blue-white crescent with a faint edge, craters and a halo. */
    component Moon: Shape {
        id: moon

        required property var spec
        readonly property real cx: spec.x
        readonly property real cy: spec.y
        readonly property real r: spec.r
        readonly property string crescent: root.crescentPath(cx, cy, r,
                                                            cx + 0.45 * r * Math.SQRT1_2,
                                                            cy - 0.45 * r * Math.SQRT1_2,
                                                            0.95 * r)

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: moon.cx
                centerY: moon.cy
                centerRadius: moon.r * 2.1

                GradientStop { position: 0; color: "#00bfe6ff" }
                GradientStop { position: 0.47; color: "#00bfe6ff" }
                GradientStop { position: 0.64; color: "#44bfe6ff" }
                GradientStop { position: 1; color: "#00bfe6ff" }
            }
            PathSvg { path: root.circle(moon.cx, moon.cy, moon.r * 2.1) }
        }
        ShapePath {
            strokeColor: "#8fb4d4"
            strokeWidth: moon.r * 0.09
            fillGradient: RadialGradient {
                centerX: moon.cx - 0.3 * moon.r
                centerY: moon.cy + 0.2 * moon.r
                centerRadius: moon.r * 1.1

                GradientStop { position: 0; color: "#ffffff" }
                GradientStop { position: 1; color: "#bcd8ee" }
            }
            PathSvg { path: moon.crescent }
        }
        ShapePath {
            // Soft craters, inside the lit part of the crescent.
            strokeColor: "transparent"
            fillColor: "#8fb3d3"
            PathSvg {
                path: root.circle(moon.cx - 0.42 * moon.r, moon.cy + 0.36 * moon.r, 0.2 * moon.r)
                    + root.circle(moon.cx - 0.12 * moon.r, moon.cy + 0.66 * moon.r, 0.13 * moon.r)
                    + root.circle(moon.cx - 0.66 * moon.r, moon.cy + 0.02 * moon.r, 0.12 * moon.r)
            }
        }
    }

    /*! A twinkle: a soft white halo and a four-point star. */
    component Star: Shape {
        id: star

        required property var spec

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillGradient: RadialGradient {
                centerX: star.spec.x
                centerY: star.spec.y
                centerRadius: star.spec.r * 2.6

                GradientStop { position: 0; color: "#8cffffff" }
                GradientStop { position: 1; color: "#00ffffff" }
            }
            PathSvg { path: root.circle(star.spec.x, star.spec.y, star.spec.r * 2.6) }
        }
        ShapePath {
            fillColor: "#ffffff"
            strokeColor: "#b8dcf2"
            strokeWidth: star.spec.r * 0.12
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.starPath(star.spec.x, star.spec.y, star.spec.r) }
        }
    }

    /*! A glossy Wii-blue raindrop with a white streak. */
    component Drop: Shape {
        id: drop

        required property var spec

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#0a6ea6"
            strokeWidth: 0.25
            joinStyle: ShapePath.RoundJoin
            fillGradient: LinearGradient {
                x1: drop.spec.x
                y1: drop.spec.top
                x2: drop.spec.x
                y2: drop.spec.top + drop.spec.h

                GradientStop { position: 0; color: "#00a9e0" }
                GradientStop { position: 1; color: "#0078b5" }
            }
            PathSvg { path: root.dropPath(drop.spec) }
        }
        ShapePath {
            fillColor: "transparent"
            strokeColor: "#e6ffffff"
            strokeWidth: drop.spec.w / 2 * 0.34
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.dropStreak(drop.spec) }
        }
    }

    /*! A snowflake: white over a soft blue edge. */
    component Flake: Shape {
        id: flake

        required property var spec

        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#8ccbeb"
            fillColor: "transparent"
            strokeWidth: flake.spec.r * 0.46
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.flakePath(flake.spec.x, flake.spec.y, flake.spec.r) }
        }
        ShapePath {
            strokeColor: "#ffffff"
            fillColor: "transparent"
            strokeWidth: flake.spec.r * 0.24
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.flakePath(flake.spec.x, flake.spec.y, flake.spec.r) }
        }
    }

    /*! The bolt: golden, with a white core. */
    component Bolt: Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#e89a00"
            strokeWidth: 0.8
            joinStyle: ShapePath.RoundJoin
            fillGradient: LinearGradient {
                x1: 38
                y1: 30
                x2: 38
                y2: 60

                GradientStop { position: 0; color: "#ffe874" }
                GradientStop { position: 1; color: "#ffb300" }
            }
            PathSvg { path: root.boltPath(1) }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: "#e6ffffff"
            PathSvg { path: root.boltPath(0.5) }
        }
    }

    /*! A fog or mist band: a white capsule with a faint blue edge. */
    component Band: Shape {
        id: band

        required property var spec

        anchors.fill: parent
        opacity: band.spec.a
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#9fbfd8"
            fillColor: "transparent"
            strokeWidth: band.spec.w + 1.5
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.line(band.spec.x1, band.spec.y, band.spec.x2, band.spec.y) }
        }
        ShapePath {
            strokeColor: "#ffffff"
            fillColor: "transparent"
            strokeWidth: band.spec.w
            capStyle: ShapePath.RoundCap
            PathSvg { path: root.line(band.spec.x1, band.spec.y, band.spec.x2, band.spec.y) }
        }
    }

    /*! The question mark of the unknown condition. */
    component Mark: Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "#0078b5"
            fillColor: "transparent"
            strokeWidth: 2.6
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.questionPath() }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: "#0078b5"
            PathSvg { path: root.circle(28, 38.5, 1.9) }
        }
    }
}
