/*
 * Commodore 64 LightCycle: today's brightness curve as a PETSCII bar chart, one
 * column per character cell. Each column is the target brightness, drawn with
 * eighth-block characters (▁..█) from the bottom; the weather cost is the cyan
 * part above it; the column of now is white. Hours run along the foot. Static:
 * no animation. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat
import "Petscii.js" as Petscii

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property var curves: client.dayCurve && client.dayCurve.monitors ? client.dayCurve.monitors : []
    readonly property bool available: client.isConnected && curves.length > 0 && curves[0].samples.length > 1
    readonly property var samples: available ? curves[0].samples : []
    readonly property real t0: available ? samples[0].t : 0
    readonly property real t1: available ? samples[samples.length - 1].t : 1
    /*! Plot height in cells. */
    readonly property int rows: 6

    C64Style {
        id: style

        theme: root.theme
    }

    /*! The plot as `rows` strings per layer, one character per column:
     *  fg (target), cost (weather, cyan), now (the column of the present, white). */
    function plot(cols, nowEpoch) {
        const out = { fg: [], cost: [], now: [] };
        if (!available || cols < 1 || t1 <= t0) {
            return out;
        }
        const span = t1 - t0;
        const nowCol = nowEpoch >= t0 && nowEpoch <= t1
            ? Math.min(cols - 1, Math.floor((nowEpoch - t0) / span * cols)) : -1;
        const target = [];
        const total = [];
        for (let c = 0; c < cols; ++c) {
            const t = t0 + (c + 0.5) * span / cols;
            const pct = Math.max(0, Math.min(100, Petscii.valueAt(samples, t, "pct")));
            const weather = Petscii.valueAt(samples, t, "noWeather");
            const withoutWeather = isFinite(weather) ? Math.max(pct, Math.min(100, weather)) : pct;
            target.push(Math.round(pct / 100 * rows * 8));
            total.push(Math.round(withoutWeather / 100 * rows * 8));
        }
        for (let r = 0; r < rows; ++r) {
            const base = (rows - 1 - r) * 8;
            let fg = "";
            let cost = "";
            let now = "";
            for (let c = 0; c < cols; ++c) {
                const lp = Math.max(0, Math.min(8, target[c] - base));
                const lt = Math.max(0, Math.min(8, total[c] - base));
                if (c === nowCol) {
                    now += Petscii.blocks(lp > 0 ? lp : lt);
                    fg += " ";
                    cost += " ";
                } else {
                    now += " ";
                    fg += lp > 0 ? Petscii.blocks(lp) : " ";
                    cost += lp === 0 && lt > 0 ? Petscii.blocks(lt) : " ";
                }
            }
            out.fg.push(fg);
            out.cost.push(cost);
            out.now.push(now);
        }
        return out;
    }

    /*! Hour labels (every six local hours) under their columns. */
    function axis(cols) {
        const chars = [];
        for (let i = 0; i < cols; ++i) {
            chars.push(" ");
        }
        if (!available || t1 <= t0) {
            return chars.join("");
        }
        const d = new Date(t0 * 1000);
        d.setMinutes(0, 0, 0);
        for (let k = 0; k < 40; ++k) {
            const e = d.getTime() / 1000;
            if (e > t1) {
                break;
            }
            if (e >= t0 && d.getHours() % 6 === 0) {
                const col = Math.floor((e - t0) / (t1 - t0) * cols);
                const label = PixelFormat.hour(d.getHours());
                for (let j = 0; j < label.length && col + j < cols; ++j) {
                    chars[col + j] = label[j];
                }
            }
            d.setHours(d.getHours() + 1);
        }
        return chars.join("");
    }

    /*! The target brightness now, as "39%", or "--" outside the curve. */
    readonly property string nowLabel: {
        if (!available || client.nowEpochS < t0 || client.nowEpochS > t1) {
            return "--";
        }
        return Math.round(Petscii.valueAt(samples, client.nowEpochS, "pct")) + "%";
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Graphic
    Accessible.name: i18ndc(settings.domain, "Accessible name of the light cycle plot", "Light cycle")
    Accessible.description: nowLabel

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + 2 * style.margin
        regionName: "c64-curve"

        readonly property int cols: style.cols(artWidth)
        readonly property var plotRows: root.plot(cols, root.client.nowEpochS)
        readonly property string axisLine: root.axis(cols)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.margin
            width: surface.cols * style.cw

            Row {
                width: body.width

                C64Text {
                    id: curveTitle

                    ink: style.ink
                    text: "LOAD \"DAYLIGHT\",8,1"
                }
                Item {
                    width: Math.max(0, body.width - curveTitle.width - nowText.width)
                    height: style.ch
                }
                C64Text {
                    id: nowText

                    ink: style.hot
                    text: "NOW " + root.nowLabel
                }
            }
            Repeater {
                model: surface.plotRows.fg.length

                delegate: Item {
                    id: plotLine

                    required property int index

                    width: body.width
                    height: style.ch

                    C64Text {
                        ink: style.ink
                        text: surface.plotRows.fg[plotLine.index]
                    }
                    C64Text {
                        ink: style.cyan
                        text: surface.plotRows.cost[plotLine.index]
                    }
                    C64Text {
                        ink: style.hot
                        text: surface.plotRows.now[plotLine.index]
                    }
                }
            }
            C64Text {
                ink: style.quiet
                text: surface.axisLine
            }
        }
    }
}
