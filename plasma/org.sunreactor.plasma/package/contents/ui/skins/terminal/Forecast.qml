/*
 * Terminal skin of the forecast module: the next intervals the daemon
 * fetched for its own policy, as five columns with a sparkline of their
 * temperatures across the strip.
 *
 *     ╭ Forecast ─────────────────────────── next 5 h ╮
 *     │ 3:00 PM │ 4:00 PM │ 5:00 PM │ 6:00 PM │ 7:00 PM │
 *     │   ✶▗█   │   ▟█▙   │   ▟█▙   │   ▞▞▞   │   ✶▗█   │   symbol per column
 *     │   15°   │   14°   │   13°   │   13°   │   12°   │
 *     │ ▁▂▃▄▅▅▆▇▇█▇▆▅▄▃▂▁▁▂▂▃▄▅▆▅▄▃▂▁▂▃▄▅▆▇▇▆▅▄▃▂▁     │   sparkline, every cell
 *     ╰───────────────────────────────────────────────╯
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../components"
import "../../Format.js" as Format
import "Term.js" as Term
import "TermArt.js" as Art
import "../../modules/ForecastRules.js" as ForecastRules

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain

    TermMetrics {
        id: term
        theme: root.theme
    }

    WeatherText {
        id: weatherText
    }

    /*! The next five intervals after now (the daemon's forecast order). */
    readonly property int slotCount: 5
    readonly property var points: ForecastRules.upcoming(client.forecast, client.nowEpochS, slotCount)
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled && points.length > 0

    /*! Column geometry in cells: `start` and `width` of each, one cell between columns. */
    readonly property var slots: {
        const n = points.length;
        const out = [];
        if (n === 0) {
            return out;
        }
        const room = Math.max(n, frame.innerCols - (n - 1));
        const base = Math.floor(room / n);
        const extra = room - base * n;
        let at = 0;
        for (let i = 0; i < n; ++i) {
            const width = base + (i < extra ? 1 : 0);
            out.push({ start: at, width: width, centre: at + (width - 1) / 2 });
            at += width + 1;
        }
        return out;
    }
    /*! The narrowest column, in cells: the time labels are 12-hour ("3:00 PM")
        when every one fits it, else 24-hour ("15:00"), which always does. */
    readonly property int minSlotCells: slots.length > 0 ? Math.min(...slots.map(s => s.width)) : 0
    readonly property bool longTimes: points.every(p => Term.len(Format.timeText(p.epochS)) <= minSlotCells)
    function clockLabel(epochS) {
        return longTimes ? Format.timeText(epochS) : Format.timeShortText(epochS);
    }
    /*! The sparkline: one level per cell, through the temperature of each column. */
    readonly property string spark: Art.sparkline(points.map(p => p.temperatureC), frame.innerCols,
                                                 slots.map(s => s.centre))

    function nightAt(epochS) {
        if (!(client.sunriseEpochS > 0 && client.sunsetEpochS > 0)) {
            return false;
        }
        return epochS < client.sunriseEpochS || epochS > client.sunsetEpochS;
    }

    implicitHeight: frame.implicitHeight

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        regionName: "forecast"
        title: i18nd(root.domain, "Forecast")
        rightTitle: root.points.length > 0
            ? i18ndp(root.domain, "next hour", "next %1 h", Math.max(1, Math.round((root.points[root.points.length - 1].epochS - root.client.nowEpochS) / 3600)))
            : ""
        contentRows: 6

        // Column separators, rows 0-4 (the sparkline row runs under them).
        Repeater {
            model: root.slots.length > 1 ? root.slots.slice(1) : []

            delegate: Rectangle {
                required property var modelData

                x: term.lineX(modelData.start - 1)
                y: term.cy(0)
                width: term.lw
                height: term.cy(5)
                color: root.theme.borderInactive
                antialiasing: false
            }
        }

        Repeater {
            model: root.points

            delegate: Item {
                id: slot

                required property var modelData
                required property int index

                readonly property var geometry: root.slots[index]

                x: term.cx(geometry.start)
                width: term.cx(geometry.width)
                height: term.cy(5)

                TermText {
                    id: timeLabel

                    tm: term
                    y: term.cy(0)
                    x: term.cx(Math.max(0, Math.floor((slot.geometry.width - usedCells) / 2)))
                    value: root.clockLabel(slot.modelData.epochS)
                    cells: slot.geometry.width
                    color: root.theme.textMuted
                }

                // The symbol, three rows high, or the chosen Weather icons style in its place.
                ThemedWeatherIcon {
                    x: Math.round((slot.width - size) / 2)
                    y: term.cy(1)
                    theme: root.theme
                    condition: slot.modelData.condition
                    night: root.nightAt(slot.modelData.epochS)
                    size: Math.min(term.cy(3), slot.width)

                    TerminalIcon {
                        anchors.fill: parent
                        theme: root.theme
                        condition: slot.modelData.condition
                        night: root.nightAt(slot.modelData.epochS)
                        size: parent.width
                    }
                }

                TermText {
                    id: tempLabel

                    tm: term
                    y: term.cy(4)
                    x: term.cx(Math.max(0, Math.floor((slot.geometry.width - usedCells) / 2)))
                    value: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                  Format.tempRounded(slot.modelData.temperatureC, root.client.isFahrenheit))
                    cells: slot.geometry.width
                    strong: true
                    color: root.theme.fg
                }
            }
        }

        // The sparkline, one cell per level, across the whole strip (row 5).
        TermPicture {
            visible: root.spark.length > 0
            tm: term
            theme: root.theme
            y: term.cy(5)
            rows: [root.spark]
            roles: [root.spark.replace(/./gu, "a")]
            colours: ({ a: root.theme.mix(root.theme.bg, root.theme.secondaryAccent, 0.55) })
        }
    }
}
