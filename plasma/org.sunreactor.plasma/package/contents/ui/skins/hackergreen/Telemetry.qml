/*
 * hackergreen status line, above the tabs: a shell prompt, then the daemon's
 * state as one line of output. The cursor is a static block, never a blink.
 *
 *   root@sunreactor:~$ sunreactor --status
 *   [ OK ] mi 39% · len 9% · 22.1°C · 14:32 █
 *
 * The flag carries the state in words and colour (red offline, amber manual,
 * paused or idle). Segments that do not fit are dropped, never clipped.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "../instrument"
import "../terminal"
import "../terminal/Term.js" as Term
import "Hg.js" as Hg

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
    HgCrt {
        theme: root.theme
    }

    readonly property var badge: {
        const t = root.theme;
        if (!client.isConnected || client.mode === SunReactorClient.Offline) {
            return Hg.flag(t, "err", i18ndc(domain, "Daemon state in the status line", "OFFLINE"));
        }
        switch (client.mode) {
        case SunReactorClient.Paused:
            return Hg.flag(t, "warn", i18ndc(domain, "Daemon state in the status line", "PAUSED"));
        case SunReactorClient.Manual:
            return Hg.flag(t, "warn", i18ndc(domain, "Daemon state in the status line", "MANUAL"));
        case SunReactorClient.IdleDimmed:
            return Hg.flag(t, "warn", i18ndc(domain, "Daemon state in the status line", "IDLE"));
        default:
            return Hg.flag(t, "ok", i18ndc(domain, "Daemon state in the status line", "OK"));
        }
    }

    readonly property var segments: {
        const out = [];
        if (client.isConnected) {
            client.monitors.forEach(m => {
                out.push([{ t: Term.shortName(m.logicalId).toLowerCase() + " ", c: root.theme.textMuted },
                          { t: m.appliedPercent >= 0 ? i18ndc(domain, "Brightness percentage", "%1%", m.appliedPercent) : "—",
                            c: root.theme.fg, b: true }]);
            });
            if (client.hasTemperature) {
                out.push([{ t: i18ndc(domain, "Temperature in degrees Celsius, one decimal", "%1°C",
                                      client.temperatureC.toFixed(1)), c: root.theme.fg }]);
            }
        }
        if (client.nowEpochS > 0) {
            out.push([{ t: Format.timeText(client.nowEpochS), c: root.theme.fg, b: true }]);
        }
        return out;
    }

    readonly property int cols: term.cols(root.width)
    /*! The output line: the badge, the segments that fit, and the static cursor. */
    readonly property var outputRuns: {
        const cursorCells = 2;
        let line = [root.badge];
        let used = Term.len(root.badge.t);
        for (const segment of root.segments) {
            const w = segment.reduce((n, r) => n + Term.len(r.t), 0) + 3;
            if (used + w + cursorCells > root.cols) {
                continue;
            }
            used += w;
            line.push({ t: " · ", c: root.theme.borderInactive });
            line = line.concat(segment);
        }
        return line.concat([{ t: " █", c: root.theme.fg }]);
    }

    implicitHeight: term.cy(2)

    Accessible.role: Accessible.StaticText
    Accessible.name: outputRuns.map(r => r.t).join("")

    Glow {
        anchors.fill: lit
        source: lit
        strength: root.theme.glow
    }

    Item {
        id: lit

        width: root.width
        height: term.cy(2)

        TermRuns {
            tm: term
            cells: root.cols
            defaultColor: root.theme.fg
            runs: [{ t: "root@sunreactor", c: root.theme.fg, b: true },
                   { t: ":~", c: root.theme.textMuted },
                   { t: "$ ", c: root.theme.textMuted },
                   { t: "sunreactor --status", c: root.theme.accent }]
        }

        TermRuns {
            tm: term
            y: term.cy(1)
            cells: root.cols
            defaultColor: root.theme.fg
            runs: root.outputRuns
        }
    }
}
