/*
 * Terminal skin of the telemetry module: the TUI's status line, centred
 * (chrome.rs status_segments):
 *
 *     ● Live · Mi 5% · Len 9% · 15.1°C · 06:30
 *
 * The badge carries the daemon state (success / warning / error, with a
 * glyph so colour is never the only signal); display names are muted, their
 * applied level bold. Segments that do not fit are dropped, never clipped.
 * Unlike the vector module it stays visible offline ("○ Offline").
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
    readonly property bool available: true

    TermMetrics {
        id: term
        theme: root.theme
    }

    readonly property var badge: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return [{ t: "○ " + i18ndc(domain, "Daemon state in the status line", "Offline"), c: theme.error, b: true }];
        case SunReactorClient.Paused:
            return [{ t: "▲ " + i18ndc(domain, "Daemon state in the status line", "Suspended"), c: theme.warning, b: true }];
        case SunReactorClient.Manual:
            return [{ t: "● ", c: theme.warning }, { t: i18ndc(domain, "Daemon state in the status line", "Manual"), c: theme.warning, b: true }];
        case SunReactorClient.IdleDimmed:
            return [{ t: "▲ " + i18ndc(domain, "Daemon state in the status line", "Idle dimmed"), c: theme.warning, b: true }];
        default:
            return [{ t: "● ", c: theme.success }, { t: i18ndc(domain, "Daemon state in the status line", "Live"), c: theme.success }];
        }
    }

    readonly property var segments: {
        const out = [];
        if (client.isConnected) {
            const names = client.monitors.map(m => Term.shortName(m.logicalId));
            const unique = names.every((n, i) => names.indexOf(n) === i);
            client.monitors.forEach((m, i) => {
                const label = unique ? names[i] : Term.fit(m.logicalId, 12);
                out.push([{ t: label + " ", c: theme.textMuted },
                          { t: m.appliedPercent >= 0 ? i18ndc(domain, "Brightness percentage", "%1%", m.appliedPercent) : "—",
                            c: theme.fg, b: true }]);
            });
            if (client.hasTemperature) {
                out.push([{ t: i18ndc(domain, "Temperature in degrees Celsius, one decimal", "%1°C",
                                      client.temperatureC.toFixed(1)), c: theme.fg }]);
            }
        }
        if (client.nowEpochS > 0) {
            out.push([{ t: Format.timeText(client.nowEpochS), c: theme.fg, b: true }]);
        }
        return out;
    }

    readonly property int cols: term.cols(width)
    readonly property var runs: {
        let line = badge.slice();
        let used = line.reduce((n, r) => n + Term.len(r.t), 0);
        for (const segment of segments) {
            const w = segment.reduce((n, r) => n + Term.len(r.t), 0) + 3;
            if (used + w > cols) {
                continue;
            }
            used += w;
            line.push({ t: " · ", c: theme.borderInactive });
            line = line.concat(segment);
        }
        return line;
    }

    implicitHeight: term.ch

    Accessible.role: Accessible.StaticText
    Accessible.name: runs.map(r => r.t).join("")

    TermRuns {
        tm: term
        x: term.inset(root.width) + term.cx(Math.floor((root.cols - usedCells) / 2))
        runs: root.runs
        cells: root.cols
        defaultColor: root.theme.fg
    }
}
