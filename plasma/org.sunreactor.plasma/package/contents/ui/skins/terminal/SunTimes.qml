/*
 * Terminal skin of the sun times module: the anchors row of the TUI's light
 * cycle panel (light_cycle.rs render_events) in a "Sun today" box.
 *
 *     ╭ Sun today ──────────── Daylight 11h 25m ╮
 *     │   ↑ Sunrise   │ ✶ Solar noon │  ↓ Sunset │
 *     │    7:13 AM    │   12:55 PM   │  6:38 PM  │
 *     │ Sun now 38.0° above the horizon         │   (when enabled)
 *     ╰─────────────────────────────────────────╯
 *
 * The next event is in the accent colour.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Term.js" as Term

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property var entries: [
        { icon: "↑", label: i18nd(domain, "Sunrise"), epoch: client.sunriseEpochS },
        { icon: "✶", label: i18nd(domain, "Solar noon"), epoch: client.solarNoonEpochS },
        { icon: "↓", label: i18nd(domain, "Sunset"), epoch: client.sunsetEpochS },
    ]
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    TermMetrics {
        id: term
        theme: root.theme
    }

    implicitHeight: frame.implicitHeight

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        regionName: "suntimes"
        title: i18nd(root.domain, "Sun today")
        rightRuns: [{ t: i18ndc(root.domain, "Length of the day, followed by a duration", "Daylight") + " ", c: root.theme.textMuted },
                    { t: Term.duration(root.client.sunsetEpochS - root.client.sunriseEpochS), c: root.theme.fg, b: true }]
        contentRows: root.showElevation ? 3 : 2

        Repeater {
            model: root.entries

            delegate: Item {
                id: entry

                required property var modelData
                required property int index

                readonly property int colWidth: Math.floor(frame.innerCols / 3)
                readonly property int start: index * colWidth
                readonly property int cells: index === 2 ? frame.innerCols - 2 * colWidth : colWidth
                readonly property bool isNext: modelData.epoch === root.nextEpoch
                // Columns after the first start with their │ separator.
                readonly property int lead: index > 0 ? 1 : 0
                readonly property int room: cells - lead

                x: term.cx(start)
                width: term.cx(cells)
                height: term.cy(2)

                TermRuns {
                    tm: term
                    x: term.cx(entry.lead + Math.max(0, Math.floor((entry.room - usedCells) / 2)))
                    cells: Math.max(0, entry.room)
                    runs: [{ t: entry.modelData.icon + " ", c: root.theme.accent, b: true },
                           { t: entry.modelData.label, c: root.theme.textMuted }]
                }
                TermText {
                    tm: term
                    y: term.ch
                    x: term.cx(entry.lead + Math.max(0, Math.floor((entry.room - usedCells) / 2)))
                    value: entry.modelData.epoch > 0 ? Format.timeText(entry.modelData.epoch) : "—"
                    cells: Math.max(0, entry.room)
                    strong: true
                    color: entry.isNext ? root.theme.accent : root.theme.fg
                }
                // │ between the columns.
                Rectangle {
                    visible: entry.index > 0
                    x: term.lineX(0)
                    width: term.lw
                    height: term.cy(2)
                    color: root.theme.borderInactive
                    antialiasing: false
                }
            }
        }

        TermRuns {
            visible: root.showElevation
            tm: term
            y: term.cy(2)
            cells: frame.innerCols
            runs: [{ t: i18ndc(root.domain, "Label before the sun's elevation angle", "Sun now") + " ", c: root.theme.textMuted },
                   { t: root.client.solarElevation.toFixed(1) + "°", c: root.theme.fg, b: true },
                   { t: " " + (root.client.solarElevation >= 0 ? i18nd(root.domain, "above the horizon")
                                                                 : i18nd(root.domain, "below the horizon")), c: root.theme.textMuted }]
        }
    }
}
