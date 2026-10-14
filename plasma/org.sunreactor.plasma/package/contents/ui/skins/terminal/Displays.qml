/*
 * Terminal skin of the displays module: the TUI's monitor list and range
 * controls (src/tui/ui/monitors.rs) as one panel. Every control is a full-width
 * block of four rows:
 *
 *     ╭ Displays ─────────────────────────── 1 of 2 ╮
 *     │ ❯ All displays                           44% │   name and value; inverse video
 *     │ ████████████████████████████▌░░░░░░░░░░░░░░░ │   two rows: the bar, a head cell
 *     │ ████████████████████████████▌░░░░░░░░░░░░░░░ │   press or drag anywhere on it
 *     │ ● Auto · range 1–50% · target 39%            │   state and range, muted
 *     │ mi-monitor                               39% │
 *     │ ██████████████▌░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
 *     │ ██████████████▌░░░░░░░░░░░░░░░░░░░░░░░░░░░░░ │
 *     │ ● Auto · range 1–50% · target 39%            │
 *     ╰ ↑↓ select  ←→ adjust  ⏎ range  a auto       ╯
 *
 * Keyboard: ↑/↓ move the cursor (and leave the panel at either end), ←/→
 * step 1 % (Shift or PgUp/PgDn 10 %), Home / End go to the range's ends (0 and
 * 100 % for all displays), Enter opens the range, A / Backspace returns to
 * automatic, Esc closes the range. Mouse: press or drag a bar, click a name row
 * to select it, click the range text or "a auto". Unknown stays "—" with an
 * empty, inert bar. Writes are throttled (DDC/CI dislikes packet storms).
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
    /*! Rows per control: name and value, the bar (two rows), the state line. */
    readonly property int controlRows: 4
    /*! An open range editor: its track and the two boundary fields. */
    readonly property int rangeRows: 3

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

    /*! The controls, top to bottom: mon -1 is "All displays", else an index. */
    readonly property var rows: {
        const out = [];
        if (showAll) {
            out.push({ mon: -1 });
        }
        monitors.forEach((m, i) => out.push({ mon: i }));
        return out;
    }
    readonly property int openRanges: monitors.filter(m => expanded[m.logicalId] === true).length
    readonly property int contentRows: Math.max(1, rows.length * controlRows + openRanges * rangeRows)

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
    /*! Writes the value of the cursor's bar (or the range boundary it is on). */
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
    /*! Home / End: the bottom or top of the bar's range (0 and 100 for all). */
    function jump(top) {
        if (!cursor || cursor.field !== "value") {
            return;
        }
        if (cursor.mon < 0) {
            if (client.globalPercent >= 0) {
                setPct("*", top ? 100 : 0);
            }
            return;
        }
        const m = monitors[cursor.mon];
        if (controllable(m) && m.percent >= 0) {
            const lim = shownLimits(m);
            setPct(m.logicalId, top ? lim[1] : lim[0]);
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
        case Qt.Key_Home:
            root.jump(false);
            break;
        case Qt.Key_End:
            root.jump(true);
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
        contentRows: root.contentRows

        Column {
            id: column

            width: frame.innerCols > 0 ? term.cx(frame.innerCols) : 0

            // --- one block per control: All displays, then each display -------------
            Repeater {
                model: root.rows

                delegate: Column {
                    id: block

                    required property var modelData
                    required property int index

                    readonly property int mon: modelData.mon
                    readonly property bool isAll: mon < 0
                    readonly property var m: isAll ? null : root.monitors[mon]
                    readonly property string key: isAll ? "*" : m.logicalId
                    readonly property bool canWrite: isAll ? root.client.globalPercent >= 0 : root.controllable(m)
                    readonly property int shown: isAll ? root.shownPct("*", root.client.globalPercent)
                                                       : root.shownPct(m.logicalId, m.percent)
                    readonly property var limits: isAll ? [0, 100] : root.shownLimits(m)
                    readonly property int target: isAll ? -1 : root.targetNow(m.logicalId)
                    readonly property bool manual: isAll ? root.client.isOverrideActive : m.hasOverride === true
                    readonly property bool open: !isAll && root.expanded[m.logicalId] === true
                    readonly property bool unavailable: !isAll && (m.unreachable || m.topology === "temporarily_unavailable")
                    readonly property bool focusedHere: root.activeFocus && root.isCursor(mon, "value")
                    readonly property color fillColor: !canWrite ? root.theme.textMuted
                        : (manual ? root.theme.warning : root.theme.accent)

                    width: column.width

                    // Line 1: the name, the value at the right end. Focused: inverse.
                    Item {
                        width: block.width
                        height: term.ch

                        Rectangle {
                            visible: block.focusedHere
                            anchors.fill: parent
                            color: root.theme.accent
                            antialiasing: false
                        }
                        // A hairline on the top edge between blocks: no row of its own.
                        Rectangle {
                            visible: block.index > 0
                            x: 0
                            y: 0
                            width: parent.width
                            height: term.lw
                            color: root.theme.borderInactive
                            antialiasing: false
                        }
                        TermText {
                            tm: term
                            visible: root.isCursor(block.mon, "value")
                            value: root.activeFocus ? "❯" : "›"
                            strong: root.activeFocus
                            color: block.focusedHere ? root.theme.onColor(root.theme.accent)
                                : (root.activeFocus ? root.theme.accent : root.theme.textMuted)
                        }
                        TermText {
                            tm: term
                            x: term.cx(2)
                            value: block.isAll ? i18nd(root.domain, "All displays") : block.m.logicalId
                            cells: Math.max(0, frame.innerCols - 8)
                            strong: true
                            color: block.focusedHere ? root.theme.onColor(root.theme.accent)
                                : (block.isAll || block.m.enabled ? root.theme.fg : root.theme.textMuted)
                        }
                        TermText {
                            tm: term
                            x: term.cx(Math.max(0, frame.innerCols - 6))
                            value: Term.padStart(block.shown >= 0 ? block.shown + "%" : "—", 6)
                            strong: true
                            color: block.focusedHere ? root.theme.onColor(root.theme.accent)
                                : (block.manual ? root.theme.warning : root.theme.fg)
                        }
                        MouseArea {
                            anchors.fill: parent
                            onClicked: root.select(block.mon, "value")
                        }
                    }

                    // Lines 2 and 3: the bar, the whole block is its hit area.
                    TermSlider {
                        tm: term
                        theme: root.theme
                        cells: frame.innerCols
                        value: block.shown
                        marker: block.manual && block.target >= 0 ? block.target : -1
                        interactive: block.canWrite && block.shown >= 0
                        fillColor: block.fillColor
                        onMoved: value => {
                            root.select(block.mon, "value");
                            root.setPct(block.key, value);
                        }
                        onReleased: value => {
                            root.setPct(block.key, value);
                            root.flush();
                        }
                    }

                    // Line 4: the state, the range and the target, muted.
                    Item {
                        width: block.width
                        height: term.ch

                        TermRuns {
                            id: detail

                            tm: term
                            cells: frame.innerCols
                            defaultColor: root.theme.textMuted
                            runs: {
                                const t = root.theme;
                                const out = [];
                                if (block.isAll) {
                                    out.push({ t: "● ", c: block.manual ? t.warning : t.success });
                                    out.push({ t: block.manual ? i18ndc(root.domain, "This brightness was set by hand", "Manual")
                                                                : i18ndc(root.domain, "Brightness follows the automatic curve", "Auto"),
                                               c: block.manual ? t.warning : t.success, b: true });
                                    out.push({ t: " · ", c: t.borderInactive, shed: 1 });
                                    out.push({ t: i18ndc(root.domain, "Applies to every display at once", "every display"), c: t.textMuted, shed: 1 });
                                    if (block.manual) {
                                        out.push({ t: "   " });
                                        out.push({ t: "a", c: autoHit.containsMouse ? t.accent : t.fg, b: true, k: "auto" });
                                        out.push({ t: " " + i18ndc(root.domain, "Key hint: back to automatic brightness", "auto"),
                                                   c: autoHit.containsMouse ? t.accent : t.textMuted, k: "auto2" });
                                    }
                                    return Term.shedRuns(out, frame.innerCols);
                                }
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
                                                                   Format.timeText(root.client.overrideUntilEpochS)), c: t.textMuted, shed: 1 });
                                    }
                                } else {
                                    out.push({ t: "● ", c: t.success });
                                    out.push({ t: i18ndc(root.domain, "Brightness follows the automatic curve", "Auto"), c: t.success });
                                }
                                if (m.enabled && !block.unavailable) {
                                    out.push({ t: " · ", c: t.borderInactive });
                                    out.push({ t: i18ndc(root.domain, "Brightness range limits", "range %1–%2%",
                                                         block.limits[0], block.limits[1]),
                                               c: block.open || rangeHit.containsMouse ? t.accent : t.textMuted, k: "range" });
                                    if (block.target >= 0) {
                                        out.push({ t: " · ", c: t.borderInactive, shed: 2 });
                                        out.push({ t: i18ndc(root.domain, "Placeholder is the brightness the curve targets now",
                                                             "target %1%", block.target), c: t.textMuted, shed: 2 });
                                    }
                                }
                                if (block.manual) {
                                    out.push({ t: "   " });
                                    out.push({ t: "a", c: autoHit.containsMouse ? t.accent : t.fg, b: true, k: "auto" });
                                    out.push({ t: " " + i18ndc(root.domain, "Key hint: back to automatic brightness", "auto"),
                                               c: autoHit.containsMouse ? t.accent : t.textMuted, k: "auto2" });
                                }
                                return Term.shedRuns(out, frame.innerCols);
                            }
                        }

                        MouseArea {
                            id: rangeHit

                            readonly property var span: detail.span("range")
                            visible: !block.isAll && span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                root.toggleRange(block.m);
                                root.select(block.mon, block.open ? "min" : "value");
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
                            onClicked: root.automatic(block.mon)
                        }
                    }

                    // Range editor (displays only).
                    Item {
                        visible: block.open
                        width: parent.width
                        height: term.ch

                        TermRange {
                            tm: term
                            theme: root.theme
                            x: term.cx(2)
                            cells: Math.max(10, frame.innerCols - 2)
                            minPct: block.limits[0]
                            maxPct: block.limits[1]
                            applied: block.isAll ? -1 : block.m.appliedPercent
                            focusedHandle: root.activeFocus && root.cursor && root.cursor.mon === block.mon
                                && root.cursor.field !== "value" ? root.cursor.field : ""
                            onMoved: (minPct, maxPct) => {
                                root.select(block.mon, minPct !== block.limits[0] ? "min" : "max");
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
                            current: root.isCursor(block.mon, modelData)
                            active: root.activeFocus
                            labelColor: root.theme.fg
                            onClicked: root.select(block.mon, modelData)
                            onStepped: direction => {
                                root.select(block.mon, modelData);
                                root.adjust(direction);
                            }
                        }
                    }
                }
            }
        }
    }
}
