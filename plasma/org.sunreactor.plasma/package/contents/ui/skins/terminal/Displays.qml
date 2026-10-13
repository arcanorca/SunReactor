/*
 * Terminal skin of the displays module: the TUI's monitor list and range
 * controls (src/tui/ui/monitors.rs) as one panel.
 *
 *     ╭ Displays ───────────────────── 1 of 2 ╮
 *     │   All displays                    44% │
 *     │   ▕██████████████░░░░░░░░░░░░░░░░░░▏  │
 *     │ ───────────────────────────────────── │
 *     │ ❯ mi-monitor                ‹  39% ›  │  cursor row (inverse capsule)
 *     │   ▕███████████████░░░░░░░░░░░░░░░░░▏  │  bar gauge = slider
 *     │   ╰ ● Auto · target 39% · range 1–50% │
 *     │     0 ──┃━━━━━━━━●━━━┃──────────── 100 │  (range, when opened)
 *     │     Minimum              1%           │
 *     │     Maximum             50%           │
 *     ╰ ↑↓ select  ←→ adjust  ⏎ range  a auto ╯
 *
 * Keyboard: ↑/↓ move the cursor (and leave the panel at either end), ←/→
 * step 1 % (Shift or PgUp/PgDn 10 %), Enter opens the range, A / Backspace
 * returns to automatic, Esc closes the range. Mouse: drag a gauge, drag a
 * range handle, click the range text or "a auto". Unknown stays "—" with an
 * empty, inert gauge. Writes are throttled (DDC/CI dislikes packet storms).
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
    readonly property var monitors: client.isConnected ? client.monitors : []
    readonly property bool showAll: monitors.length > 1
    readonly property bool available: client.isConnected && monitors.length > 0

    TermMetrics {
        id: term
        theme: root.theme
    }

    // --- local state -------------------------------------------------------------
    /*! logicalId -> true while that display's range is open. */
    property var expanded: ({})
    /*! Values the user asked for that the daemon has not reported back yet:
        logicalId (or "*" for all displays) -> percent. */
    property var pendingPct: ({})
    /*! logicalId -> [min, max]. */
    property var pendingLimits: ({})
    property int cursorIndex: 0

    readonly property var targets: {
        const out = [];
        if (showAll) {
            out.push({ mon: -1, field: "value" });
        }
        monitors.forEach((m, i) => {
            out.push({ mon: i, field: "value" });
            if (expanded[m.logicalId]) {
                out.push({ mon: i, field: "min" });
                out.push({ mon: i, field: "max" });
            }
        });
        return out;
    }
    readonly property var cursor: targets.length > 0 ? targets[Math.max(0, Math.min(cursorIndex, targets.length - 1))] : null

    function isCursor(mon, field) {
        return cursor !== null && cursor.mon === mon && cursor.field === field;
    }
    function select(mon, field) {
        const i = targets.findIndex(t => t.mon === mon && t.field === field);
        if (i >= 0) {
            cursorIndex = i;
        }
        root.forceActiveFocus(Qt.MouseFocusReason);
    }

    function controllable(m) {
        return m.enabled && !m.unreachable && m.topology !== "temporarily_unavailable";
    }
    function shownPct(key, reported) {
        return pendingPct[key] !== undefined ? pendingPct[key] : reported;
    }
    function shownLimits(m) {
        return pendingLimits[m.logicalId] !== undefined ? pendingLimits[m.logicalId] : [m.minPct, m.maxPct];
    }
    function setPct(key, value) {
        const next = Object.assign({}, pendingPct);
        next[key] = Math.max(0, Math.min(100, Math.round(value)));
        pendingPct = next;
        sendTimer.restart();
        settleTimer.restart();
    }
    function setLimits(m, minPct, maxPct) {
        const next = Object.assign({}, pendingLimits);
        const lo = Math.max(0, Math.min(100, Math.round(minPct)));
        next[m.logicalId] = [lo, Math.max(lo, Math.min(100, Math.round(maxPct)))];
        pendingLimits = next;
        sendTimer.restart();
        settleTimer.restart();
    }
    function flush() {
        sendTimer.stop();
        for (const key in pendingPct) {
            if (key === "*") {
                client.setGlobalOverride(pendingPct[key], settings.overrideMinutes);
            } else {
                client.setMonitorOverride(key, pendingPct[key], settings.overrideMinutes);
            }
        }
        for (const key in pendingLimits) {
            client.setMonitorLimits(key, pendingLimits[key][0], pendingLimits[key][1]);
        }
    }
    function toggleRange(m) {
        const next = Object.assign({}, expanded);
        next[m.logicalId] = !next[m.logicalId];
        expanded = next;
    }
    function automatic(mon) {
        if (mon < 0) {
            client.clearAllOverrides();
        } else {
            client.clearMonitorOverride(monitors[mon].logicalId);
        }
        pendingPct = ({});
    }
    function adjust(delta) {
        if (!cursor) {
            return;
        }
        if (cursor.mon < 0) {
            if (client.globalPercent >= 0) {
                setPct("*", shownPct("*", client.globalPercent) + delta);
            }
            return;
        }
        const m = monitors[cursor.mon];
        if (cursor.field === "value") {
            if (controllable(m) && m.percent >= 0) {
                setPct(m.logicalId, shownPct(m.logicalId, m.percent) + delta);
            }
        } else {
            const lim = shownLimits(m);
            if (cursor.field === "min") {
                setLimits(m, Math.min(lim[0] + delta, lim[1]), lim[1]);
            } else {
                setLimits(m, lim[0], Math.max(lim[1] + delta, lim[0]));
            }
        }
    }

    // The daemon's own target for a display right now (day_curve), or -1.
    function targetNow(logicalId) {
        const curve = client.dayCurve;
        if (!curve || !curve.monitors) {
            return -1;
        }
        for (const c of curve.monitors) {
            if (c.monitorId === logicalId && c.samples.length > 1) {
                const s = c.samples;
                const t = client.nowEpochS;
                if (t < s[0].t || t > s[s.length - 1].t) {
                    return -1;
                }
                for (let i = 1; i < s.length; ++i) {
                    if (s[i].t >= t) {
                        const f = (t - s[i - 1].t) / Math.max(1, s[i].t - s[i - 1].t);
                        return Math.round(s[i - 1].pct + (s[i].pct - s[i - 1].pct) * f);
                    }
                }
            }
        }
        return -1;
    }

    Timer {
        id: sendTimer
        interval: 250
        onTriggered: root.flush()
    }
    // After the last change, give the daemon time to report it back.
    Timer {
        id: settleTimer
        interval: 2500
        onTriggered: {
            root.pendingPct = ({});
            root.pendingLimits = ({});
        }
    }

    activeFocusOnTab: true
    implicitHeight: frame.implicitHeight

    Keys.onUpPressed: event => {
        if (cursorIndex > 0) {
            cursorIndex = Math.min(cursorIndex, targets.length - 1) - 1;
        } else {
            const previous = nextItemInFocusChain(false);
            if (previous) {
                previous.forceActiveFocus(Qt.BacktabFocusReason);
            }
        }
    }
    Keys.onDownPressed: event => {
        if (cursorIndex < targets.length - 1) {
            ++cursorIndex;
        } else {
            const next = nextItemInFocusChain(true);
            if (next) {
                next.forceActiveFocus(Qt.TabFocusReason);
            }
        }
    }
    Keys.onLeftPressed: event => root.adjust((event.modifiers & Qt.ShiftModifier) ? -10 : -1)
    Keys.onRightPressed: event => root.adjust((event.modifiers & Qt.ShiftModifier) ? 10 : 1)
    Keys.onPressed: event => {
        switch (event.key) {
        case Qt.Key_PageUp:
            root.adjust(10);
            break;
        case Qt.Key_PageDown:
            root.adjust(-10);
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Space:
            if (root.cursor && root.cursor.mon >= 0) {
                root.toggleRange(root.monitors[root.cursor.mon]);
            }
            break;
        case Qt.Key_A:
        case Qt.Key_Backspace:
        case Qt.Key_Delete:
            if (root.cursor) {
                root.automatic(root.cursor.mon);
            }
            break;
        case Qt.Key_Escape:
            if (root.cursor && root.cursor.mon >= 0 && root.expanded[root.monitors[root.cursor.mon].logicalId]) {
                root.toggleRange(root.monitors[root.cursor.mon]);
                root.cursorIndex = root.targets.findIndex(t => t.mon === root.cursor.mon && t.field === "value");
            } else {
                return;
            }
            break;
        default:
            return;
        }
        event.accepted = true;
    }

    function hint(key, text) {
        return [{ t: key, c: theme.fg, b: true }, { t: " " + text + "  ", c: theme.textMuted }];
    }

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        focused: root.activeFocus
        regionName: "displays"
        title: i18nd(root.domain, "Displays")
        rightTitle: {
            if (root.monitors.length < 2) {
                return "";
            }
            const mon = root.cursor && root.cursor.mon >= 0 ? root.cursor.mon : 0;
            return i18ndc(root.domain, "Position of the selected display in the list", "%1 of %2", mon + 1, root.monitors.length);
        }
        bottomRuns: root.activeFocus
            ? [].concat(root.hint("↑↓", i18ndc(root.domain, "Key hint", "select")),
                        root.hint("←→", i18ndc(root.domain, "Key hint", "adjust")),
                        root.hint("⏎", i18ndc(root.domain, "Key hint", "range")),
                        root.hint("a", i18ndc(root.domain, "Key hint: back to automatic brightness", "auto")))
            : []
        contentRows: Math.max(1, column.rows)

        Column {
            id: column

            readonly property int rows: Math.round(height * term.dpr / term.chDev)

            width: frame.innerCols > 0 ? term.cx(frame.innerCols) : 0

            // --- All displays ---------------------------------------------------
            TermField {
                visible: root.showAll
                tm: term
                theme: root.theme
                cells: frame.innerCols
                label: i18nd(root.domain, "All displays")
                value: {
                    const v = root.shownPct("*", root.client.globalPercent);
                    return v >= 0 ? v + "%" : "—";
                }
                current: root.isCursor(-1, "value")
                active: root.activeFocus
                labelColor: root.theme.fg
                onClicked: root.select(-1, "value")
                onStepped: direction => {
                    root.select(-1, "value");
                    root.adjust(direction);
                }
            }
            Item {
                visible: root.showAll
                width: parent.width
                height: term.ch

                TermBar {
                    tm: term
                    theme: root.theme
                    x: term.cx(2)
                    cells: Math.max(4, frame.innerCols - 2)
                    value: root.shownPct("*", root.client.globalPercent)
                    focused: root.activeFocus && root.isCursor(-1, "value")
                    interactive: root.client.globalPercent >= 0
                    fillColor: root.client.isOverrideActive ? root.theme.warning : root.theme.accent
                    onMoved: value => {
                        root.select(-1, "value");
                        root.setPct("*", value);
                    }
                    onReleased: value => {
                        root.setPct("*", value);
                        root.flush();
                    }
                }
            }
            // A rule under the master row, like the TUI's section headings.
            Item {
                visible: root.showAll
                width: parent.width
                height: term.ch

                Rectangle {
                    y: term.lineY(0)
                    width: parent.width
                    height: term.lw
                    color: root.theme.borderInactive
                    antialiasing: false
                }
            }

            // --- one block per display -------------------------------------------
            Repeater {
                model: root.monitors

                delegate: Column {
                    id: block

                    required property var modelData
                    required property int index

                    readonly property var m: modelData
                    readonly property bool canWrite: root.controllable(m)
                    readonly property bool open: root.expanded[m.logicalId] === true
                    readonly property int shown: root.shownPct(m.logicalId, m.percent)
                    readonly property var limits: root.shownLimits(m)
                    readonly property int target: root.targetNow(m.logicalId)
                    readonly property bool manual: m.hasOverride === true
                    readonly property bool unavailable: m.unreachable || m.topology === "temporarily_unavailable"

                    width: column.width

                    // Breathing row between displays.
                    Item {
                        visible: block.index > 0
                        width: parent.width
                        height: term.ch
                    }

                    TermField {
                        tm: term
                        theme: root.theme
                        cells: frame.innerCols
                        label: block.m.logicalId
                        value: block.shown >= 0 ? block.shown + "%" : "—"
                        current: root.isCursor(block.index, "value")
                        active: root.activeFocus
                        adjustable: block.canWrite && block.shown >= 0
                        labelColor: block.m.enabled ? root.theme.fg : root.theme.textMuted
                        valueColor: block.manual ? root.theme.warning : root.theme.fg
                        onClicked: root.select(block.index, "value")
                        onStepped: direction => {
                            root.select(block.index, "value");
                            root.adjust(direction);
                        }
                    }

                    Item {
                        width: parent.width
                        height: term.ch

                        TermBar {
                            tm: term
                            theme: root.theme
                            x: term.cx(2)
                            cells: Math.max(4, frame.innerCols - 2)
                            value: block.shown
                            marker: block.manual && block.target >= 0 ? block.target : -1
                            interactive: block.canWrite && block.shown >= 0
                            focused: root.activeFocus && root.isCursor(block.index, "value")
                            fillColor: !block.canWrite ? root.theme.textMuted
                                : (block.manual ? root.theme.warning : root.theme.accent)
                            onMoved: value => {
                                root.select(block.index, "value");
                                root.setPct(block.m.logicalId, value);
                            }
                            onReleased: value => {
                                root.setPct(block.m.logicalId, value);
                                root.flush();
                            }
                        }
                    }

                    // ╰ ● Auto · target 39% · range 1–50%   a auto
                    Item {
                        width: parent.width
                        height: term.ch

                        TermRuns {
                            id: detail

                            tm: term
                            x: term.cx(2)
                            cells: Math.max(0, frame.innerCols - 2)
                            defaultColor: root.theme.textMuted
                            runs: {
                                const t = root.theme;
                                const out = [{ t: term.detailGlyph + " ", c: t.textMuted }];
                                const m = block.m;
                                if (!m.enabled) {
                                    out.push({ t: "○ " + i18ndc(root.domain, "This display is excluded from automation", "Off"), c: t.textMuted });
                                } else if (m.unreachable) {
                                    out.push({ t: "▲ " + i18ndc(root.domain, "Display is not answering; seconds until the next try",
                                                                "Retrying in %1s", m.backoffRemainingS), c: t.warning });
                                } else if (block.unavailable) {
                                    out.push({ t: "▲ " + i18ndc(root.domain, "This display is not answering", "Unavailable"), c: t.warning });
                                } else if (block.manual) {
                                    out.push({ t: "● ", c: t.warning });
                                    out.push({ t: i18ndc(root.domain, "This brightness was set by hand", "Manual"), c: t.warning, b: true });
                                    if (root.client.overrideUntilEpochS > 0) {
                                        out.push({ t: " " + i18ndc(root.domain, "Placeholder is a time of day", "until %1",
                                                                   Format.timeText(root.client.overrideUntilEpochS)), c: t.textMuted });
                                    }
                                } else {
                                    out.push({ t: "● ", c: t.success });
                                    out.push({ t: i18ndc(root.domain, "Brightness follows the automatic curve", "Auto"), c: t.success });
                                }
                                if (block.target >= 0 && m.enabled && !block.unavailable) {
                                    out.push({ t: " · ", c: t.borderInactive });
                                    out.push({ t: i18ndc(root.domain, "Placeholder is the brightness the curve targets now",
                                                         "target %1%", block.target), c: t.textMuted });
                                }
                                out.push({ t: " · ", c: t.borderInactive });
                                out.push({ t: i18ndc(root.domain, "Brightness range limits", "range %1–%2%",
                                                     block.limits[0], block.limits[1]),
                                           c: block.open || rangeHit.containsMouse ? t.accent : t.textMuted, k: "range" });
                                if (block.manual) {
                                    out.push({ t: "  " });
                                    out.push({ t: "a", c: autoHit.containsMouse ? t.accent : t.fg, b: true, k: "auto" });
                                    out.push({ t: " " + i18ndc(root.domain, "Key hint: back to automatic brightness", "auto"),
                                               c: autoHit.containsMouse ? t.accent : t.textMuted, k: "auto2" });
                                }
                                return out;
                            }
                        }

                        MouseArea {
                            id: rangeHit

                            readonly property var span: detail.span("range")
                            visible: span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.toggleRange(block.m);
                                root.select(block.index, block.open ? "min" : "value");
                            }
                        }
                        MouseArea {
                            id: autoHit

                            readonly property var span: detail.span("auto")
                            visible: span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] + 5 : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: root.automatic(block.index)
                        }
                    }

                    // Range editor.
                    Item {
                        visible: block.open
                        width: parent.width
                        height: term.ch

                        TermRange {
                            tm: term
                            theme: root.theme
                            x: term.cx(4)
                            cells: Math.max(10, frame.innerCols - 4)
                            minPct: block.limits[0]
                            maxPct: block.limits[1]
                            applied: block.m.appliedPercent
                            focusedHandle: root.activeFocus && root.cursor && root.cursor.mon === block.index
                                && root.cursor.field !== "value" ? root.cursor.field : ""
                            onMoved: (minPct, maxPct) => {
                                root.select(block.index, minPct !== block.limits[0] ? "min" : "max");
                                root.setLimits(block.m, minPct, maxPct);
                            }
                            onReleased: (minPct, maxPct) => {
                                root.setLimits(block.m, minPct, maxPct);
                                root.flush();
                            }
                        }
                    }
                    Repeater {
                        model: block.open ? ["min", "max"] : []

                        delegate: TermField {
                            required property string modelData
                            required property int index

                            x: term.cx(2)
                            tm: term
                            theme: root.theme
                            cells: Math.max(0, frame.innerCols - 2)
                            labelCells: 12
                            label: index === 0 ? i18ndc(root.domain, "Night minimum boundary", "Minimum")
                                               : i18ndc(root.domain, "Day maximum boundary", "Maximum")
                            value: block.limits[index] + "%"
                            current: root.isCursor(block.index, modelData)
                            active: root.activeFocus
                            labelColor: root.theme.fg
                            onClicked: root.select(block.index, modelData)
                            onStepped: direction => {
                                root.select(block.index, modelData);
                                root.adjust(direction);
                            }
                        }
                    }
                }
            }
        }
    }
}
