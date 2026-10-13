/*
 * Terminal skin of the forecast module: the next intervals the daemon
 * fetched for its own policy, as the TUI's forecast panel reduced to columns.
 *
 *     ╭ Forecast ─────────────────────── next 5 h ╮
 *     │  3:00 PM  4:00 PM  5:00 PM  6:00 PM  7:00 │
 *     │    ▆▆▆      ▄▄▄                           │   temperature, two-row
 *     │    ███      ███      ▇▇▇      ▇▇▇     ▅▅▅ │   eighth-block bars
 *     │  Partly…  Cloudy    Rain    Cloudy  Partly│
 *     │    15°      14°      13°      13°     12° │
 *     ╰───────────────────────────────────────────╯
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../../Format.js" as Format
import "Term.js" as Term

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

    /*! Slots that fit: each wants 8 cells, at most 5. */
    readonly property int slots: Math.max(1, Math.min(5, Math.floor(frame.innerCols / 8)))
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === slots) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    readonly property real lowT: points.length ? Math.min(...points.map(p => p.temperatureC)) - 1 : 0
    readonly property real highT: points.length ? Math.max(...points.map(p => p.temperatureC)) + 1 : 1

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
        contentRows: 5

        Repeater {
            model: root.points

            delegate: Item {
                id: slot

                required property var modelData
                required property int index

                readonly property int w: Math.floor(frame.innerCols / root.slots)
                readonly property int barCells: Math.max(1, Math.min(3, w - 4))
                readonly property int eighths: Math.max(1, Math.round((modelData.temperatureC - root.lowT)
                                                                      / Math.max(0.1, root.highT - root.lowT) * 16))

                x: term.cx(index * w)
                width: term.cx(w)
                height: term.cy(5)

                function centred(cells) {
                    return term.cx(Math.max(0, Math.floor((w - cells) / 2)));
                }

                TermText {
                    tm: term
                    x: slot.centred(usedCells)
                    value: Format.timeText(slot.modelData.epochS)
                    cells: slot.w - 1
                    color: root.theme.textMuted
                }

                // Two rows of eighth blocks, like the TUI's ▁..█ columns.
                Rectangle {
                    x: slot.centred(slot.barCells)
                    y: term.cy(3) - height
                    width: term.cx(slot.barCells)
                    height: Math.round(slot.eighths * term.chDev / 8) / term.dpr
                    color: root.theme.mix(root.theme.warning, root.theme.bg, 0.15)
                    antialiasing: false
                }

                TermText {
                    tm: term
                    y: term.cy(3)
                    x: slot.centred(usedCells)
                    value: weatherText.conditionLabel(slot.modelData.condition) || "—"
                    cells: slot.w - 1
                    color: root.theme.textMuted
                }

                TermText {
                    tm: term
                    y: term.cy(4)
                    x: slot.centred(usedCells)
                    value: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                                  Math.round(slot.modelData.temperatureC))
                    strong: true
                    color: root.theme.fg
                }
            }
        }
    }
}
