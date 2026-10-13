/*
 * Terminal skin of the automation module: the TUI's automation panel and
 * schedule table (src/tui/ui/automation.rs) in one box.
 *
 *     ╭ Automation ─────────────────────── ● Live ╮
 *     │ ❯ Monitor   ‹ mi-monitor ›                │   with ≥2 displays
 *     │   Mode      Automatic                     │
 *     │   Next      Set 50% at 4:20 PM → 23%      │
 *     │                                           │
 *     │ Schedule ───────────────────────── ● now  │
 *     │      Event         Offset  Time    Target │
 *     │   ─────────────────────────────────────── │
 *     │   ●  Peak               —  12:55     50%  │
 *     ╰───────────────────────────────────────────╯
 *
 * In a manual override or a pause it says until when and offers the way
 * back ("[ Automatic ]", or A / Enter while focused). Offsets are not part of
 * the day_curve answer, so that column shows the TUI's "—".
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Term.js" as Term

FocusScope {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var curves: client.dayCurve && client.dayCurve.monitors ? client.dayCurve.monitors : []
    property int selected: 0
    readonly property var current: curves.length > 0 ? curves[Math.max(0, Math.min(selected, curves.length - 1))] : null
    readonly property var milestones: current ? current.milestones : []
    readonly property int currentIndex: {
        let found = -1;
        for (let i = 0; i < milestones.length; ++i) {
            if (milestones[i].t <= client.nowEpochS) {
                found = i;
            }
        }
        return found;
    }
    readonly property var nextMilestone: currentIndex + 1 < milestones.length ? milestones[currentIndex + 1] : null
    readonly property bool manual: client.mode === SunReactorClient.Manual
    readonly property bool paused: client.mode === SunReactorClient.Paused
    readonly property bool canReturn: client.isOverrideActive || paused

    readonly property bool available: client.isConnected
        && (client.mode !== SunReactorClient.Automatic || milestones.length > 0)

    TermMetrics {
        id: term
        theme: root.theme
    }

    activeFocusOnTab: curves.length > 1 || canReturn
    implicitHeight: frame.implicitHeight

    function back() {
        if (paused) {
            client.resume();
        } else {
            client.clearAllOverrides();
        }
    }
    function step(direction) {
        if (curves.length > 1) {
            selected = (Math.max(0, Math.min(selected, curves.length - 1)) + direction + curves.length) % curves.length;
        }
    }

    Keys.onLeftPressed: root.step(-1)
    Keys.onRightPressed: root.step(1)
    Keys.onPressed: event => {
        if ((event.key === Qt.Key_A || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) && root.canReturn) {
            root.back();
            event.accepted = true;
        }
    }

    readonly property var modeRuns: {
        const t = theme;
        switch (client.mode) {
        case SunReactorClient.Paused:
            return [{ t: "▲ " + i18ndc(domain, "Automation state", "Suspended"), c: t.warning, b: true },
                    { t: client.suspendUntilEpochS > 0
                         ? " · " + i18ndc(domain, "Placeholder is a time of day", "until %1", Format.timeText(client.suspendUntilEpochS))
                         : " · " + i18nd(domain, "until you resume it"), c: t.textMuted }];
        case SunReactorClient.Manual:
            return [{ t: i18ndc(domain, "Automation state", "Manual override"), c: t.warning, b: true },
                    { t: client.overrideUntilEpochS > 0
                         ? " · " + i18ndc(domain, "Placeholder is a time of day", "until %1", Format.timeText(client.overrideUntilEpochS))
                         : "", c: t.textMuted }];
        case SunReactorClient.IdleDimmed:
            return [{ t: i18ndc(domain, "Automation state", "Idle dimmed"), c: t.warning }];
        default:
            return [{ t: client.hasWeatherReading ? i18ndc(domain, "Automation state", "Automatic · sun and weather")
                                                  : i18ndc(domain, "Automation state", "Automatic"), c: t.success }];
        }
    }
    readonly property var nextRuns: nextMilestone
        ? [{ t: nextMilestone.label, c: theme.fg },
           { t: " " + i18ndc(domain, "Placeholder is a time of day, followed by an arrow and a percentage", "at %1",
                             Format.timeText(nextMilestone.t)) + " → ", c: theme.textMuted },
           { t: nextMilestone.pct >= 0 ? nextMilestone.pct + "%" : "—", c: theme.fg }]
        : [{ t: milestones.length > 0 ? i18nd(domain, "Nothing more today") : "—", c: theme.textMuted }]

    readonly property int labelCells: 10
    readonly property bool showSelector: curves.length > 1
    readonly property bool showTable: milestones.length > 0 && frame.innerCols >= 24

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        focused: root.activeFocus
        regionName: "automation"
        title: i18nd(root.domain, "Automation")
        rightRuns: {
            const t = root.theme;
            switch (root.client.mode) {
            case SunReactorClient.Automatic:
                return [{ t: "● ", c: t.success }, { t: i18ndc(root.domain, "Daemon state in the status line", "Live"), c: t.textMuted }];
            case SunReactorClient.Offline:
                return [{ t: "○ ", c: t.error }, { t: i18ndc(root.domain, "Daemon state in the status line", "Offline"), c: t.textMuted }];
            default:
                return [{ t: "▲ ", c: t.warning }, { t: i18ndc(root.domain, "Daemon state in the status line", "Attention"), c: t.textMuted }];
            }
        }
        contentRows: Math.max(1, lines.rows)

        Column {
            id: lines

            readonly property int rows: Math.round(height * term.dpr / term.chDev)

            width: term.cx(frame.innerCols)

            TermField {
                visible: root.showSelector
                tm: term
                theme: root.theme
                cells: frame.innerCols
                labelCells: root.labelCells
                valueCells: Math.min(18, Math.max(6, frame.innerCols - root.labelCells - 8))
                label: i18nd(root.domain, "Monitor")
                value: root.current ? Term.fit(root.current.monitorId, Math.min(18, Math.max(6, frame.innerCols - root.labelCells - 8))) : ""
                current: true
                active: root.activeFocus
                onClicked: root.forceActiveFocus(Qt.MouseFocusReason)
                onStepped: direction => root.step(direction)
            }

            Item {
                width: parent.width
                height: term.ch

                TermText {
                    tm: term
                    x: term.cx(2)
                    value: i18ndc(root.domain, "Label of the automation state row", "Mode")
                    cells: root.labelCells
                    color: root.theme.textMuted
                }
                TermRuns {
                    tm: term
                    x: term.cx(2 + root.labelCells + 1)
                    runs: root.modeRuns
                    cells: Math.max(0, frame.innerCols - root.labelCells - 3)
                }
            }

            Item {
                width: parent.width
                height: term.ch

                TermText {
                    tm: term
                    x: term.cx(2)
                    value: i18ndc(root.domain, "Label of the next automation step", "Next")
                    cells: root.labelCells
                    color: root.theme.textMuted
                }
                TermRuns {
                    tm: term
                    x: term.cx(2 + root.labelCells + 1)
                    runs: root.nextRuns
                    cells: Math.max(0, frame.innerCols - root.labelCells - 3)
                }
            }

            // The way back: a terminal button.
            Item {
                visible: root.canReturn
                width: parent.width
                height: term.cy(2)

                TermText {
                    id: backButton

                    readonly property bool lit: root.activeFocus || backMouse.containsMouse
                    tm: term
                    x: term.cx(2 + root.labelCells + 1)
                    y: term.ch
                    value: root.paused ? i18ndc(root.domain, "@action:button Resume automatic brightness", "[ Resume ]")
                                       : i18ndc(root.domain, "@action:button Return to automatic brightness", "[ Automatic ]")
                    strong: true
                    color: lit ? root.theme.onColor(root.theme.accent) : root.theme.accent
                    inverse: lit
                    inverseColor: root.theme.accent

                    MouseArea {
                        id: backMouse

                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: root.back()
                    }
                }
                TermRuns {
                    tm: term
                    visible: root.activeFocus
                    x: backButton.x + backButton.width + term.cw
                    y: term.ch
                    runs: [{ t: "a", c: root.theme.fg, b: true },
                           { t: " " + i18ndc(root.domain, "Key hint", "or ⏎"), c: root.theme.textMuted }]
                    cells: Math.max(0, frame.innerCols - root.labelCells - 3 - backButton.usedCells - 1)
                }
            }

            // Schedule heading: bold text, a faint rule, "● now" on the right.
            Item {
                visible: root.showTable
                width: parent.width
                height: term.cy(2)

                TermText {
                    id: heading

                    tm: term
                    y: term.ch
                    value: i18nd(root.domain, "Schedule")
                    strong: true
                    color: root.theme.fg
                }
                Rectangle {
                    x: heading.width + term.cw
                    y: term.ch + term.lineY(0)
                    width: Math.max(0, term.cx(frame.innerCols - nowKey.usedCells - 1) - x)
                    height: term.lw
                    color: root.theme.borderInactive
                    antialiasing: false
                }
                TermRuns {
                    id: nowKey

                    tm: term
                    x: term.cx(frame.innerCols - usedCells)
                    y: term.ch
                    runs: [{ t: "● ", c: root.theme.secondaryAccent },
                           { t: i18ndc(root.domain, "Legend: the dot marks the step in effect now", "now"), c: root.theme.textMuted }]
                }
            }

            TermTable {
                id: table

                visible: root.showTable
                tm: term
                theme: root.theme
                width: parent.width
                height: implicitHeight
                columns: {
                    const time = Term.len(Format.timeText(root.client.nowEpochS || 1)) + 1;
                    const cols = [{ title: i18ndc(root.domain, "Schedule column", "Event"), cells: 0 }];
                    if (frame.innerCols >= 40) {
                        cols.push({ title: i18ndc(root.domain, "Schedule column", "Offset"), cells: 6, right: true });
                    }
                    cols.push({ title: i18ndc(root.domain, "Schedule column", "Time"), cells: Math.max(5, time) });
                    cols.push({ title: i18ndc(root.domain, "Schedule column", "Target"), cells: 6, right: true });
                    return cols;
                }
                rows: root.milestones.map((m, i) => {
                    const values = [m.label];
                    if (frame.innerCols >= 40) {
                        values.push("—");
                    }
                    values.push(Format.timeText(m.t));
                    values.push(m.pct >= 0 ? m.pct + "%" : "—");
                    const colors = values.map(() => "");
                    if (frame.innerCols >= 40) {
                        colors[1] = root.theme.textMuted;
                    }
                    return { values: values, colors: colors, current: i === root.currentIndex,
                             strong: root.nextMilestone !== null && m === root.nextMilestone,
                             muted: i < root.currentIndex };
                })
            }
        }
    }
}
