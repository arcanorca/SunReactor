/*
 * hackergreen sun times: the day's anchors as a text table, the next one marked.
 *
 *   [ SUN ]                                        DAYLIGHT 11h 25m
 *   EVENT                                                      TIME
 *   ----------------------------------------------------------------
 *   ● ↑ Sunrise                                             07:13
 *     ✶ Solar noon                                          12:55
 *     ↓ Sunset                                              18:38
 *   sun now 38.0° above the horizon
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../terminal"
import "../terminal/Term.js" as Term
import "../../Format.js" as Format

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
    HgCrt {
        theme: root.theme
    }

    implicitHeight: section.implicitHeight

    HgSection {
        id: section

        tm: term
        theme: root.theme
        width: root.width
        title: i18ndc(root.domain, "Section title of the sun times", "SUN")
        bracket: true
        metaRuns: [{ t: i18ndc(root.domain, "Length of the day, followed by a duration", "DAYLIGHT") + " ",
                     c: root.theme.textMuted },
                   { t: Term.duration(root.client.sunsetEpochS - root.client.sunriseEpochS), c: root.theme.fg, b: true }]
        bodyHeight: term.cy(5 + (root.showElevation ? 1 : 0))
        glowing: true

        TermTable {
            tm: term
            theme: root.theme
            width: root.width
            columns: [{ title: i18ndc(root.domain, "Column title: the sun event", "EVENT"), cells: 0 },
                      { title: i18ndc(root.domain, "Column title: the time of the sun event", "TIME"), cells: 8, right: true }]
            rows: root.entries.map(e => ({
                values: [e.icon + " " + e.label, e.epoch > 0 ? Format.timeText(e.epoch) : "—"],
                current: e.epoch === root.nextEpoch,
                strong: e.epoch === root.nextEpoch,
            }))
            currentColor: root.theme.accent
        }

        TermRuns {
            visible: root.showElevation
            tm: term
            y: term.cy(5)
            cells: term.cols(root.width)
            runs: [{ t: i18ndc(root.domain, "Label before the sun's elevation angle", "sun now") + " ", c: root.theme.textMuted },
                   { t: root.client.solarElevation.toFixed(1) + "°", c: root.theme.fg, b: true },
                   { t: " " + (root.client.solarElevation >= 0 ? i18nd(root.domain, "above the horizon")
                                                                 : i18nd(root.domain, "below the horizon")), c: root.theme.textMuted }]
        }
    }
}
