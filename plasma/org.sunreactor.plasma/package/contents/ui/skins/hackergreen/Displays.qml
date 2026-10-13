/*
 * hackergreen displays: one ASCII gauge per display, the value at its end.
 *
 *   == DISPLAYS =====================================  1 of 2
 *   > mi-monitor [██████░░░░░░░░░░░░░░░░░░░] 39%
 *     └ [ MANUAL ] target 34% · range 1–50% · a auto
 *
 * The cursor row is the one in inverse video while the list has focus. Keys
 * are the terminal Displays skin's: ↑/↓ move, ←/→ step 1 % (Shift or PgUp/PgDn
 * 10 %), Enter opens the range, A / Backspace returns to automatic, Esc closes
 * the range. Mouse: drag a gauge or a range handle, click the range or "a auto".
 * Unknown stays "—" with an empty, inert gauge. Writes are throttled (DDC/CI
 * dislikes packet storms).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../terminal"
import "../terminal/Term.js" as Term
import "Hg.js" as Hg

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
    HgCrt {
        theme: root.theme
    }

    // --- local state -------------------------------------------------------------
    /*! logicalId -> true while that display's range is open. */
    property var expanded: ({})
    /*! Values asked for that the daemon has not reported back yet: logicalId (or "*") -> percent. */
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

    /*! The daemon's own target for a display right now (day_curve), or -1. */
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
    implicitHeight: section.implicitHeight

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

    readonly property int innerCols: term.cols(root.width)
    /*! Gauge width: the value and the label take the rest of the row. */
    readonly property int gaugeCells: Math.max(6, Math.min(40, innerCols - 21))

    HgSection {
        id: section

        tm: term
        theme: root.theme
        width: root.width
        title: i18ndc(root.domain, "Section title of the displays", "DISPLAYS")
        metaRuns: root.monitors.length < 2 ? [] : [{
            t: i18ndc(root.domain, "Position of the selected display in the list", "%1 of %2",
                      (root.cursor && root.cursor.mon >= 0 ? root.cursor.mon : 0) + 1, root.monitors.length),
            c: root.theme.textMuted }]
        bodyHeight: column.height

        Column {
            id: column

            width: root.width

            // --- All displays -------------------------------------------------------
            Item {
                id: master

                visible: root.showAll
                width: parent.width
                height: term.ch

                readonly property int shown: root.shownPct("*", root.client.globalPercent)

                HgRow {
                    tm: term
                    theme: root.theme
                    label: i18nd(root.domain, "All displays")
                    current: root.isCursor(-1, "value")
                    active: root.activeFocus
                    onClicked: root.select(-1, "value")
                }
                HgGauge {
                    tm: term
                    theme: root.theme
                    x: term.cx(15)
                    cells: root.gaugeCells
                    value: master.shown
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
                HgValue {
                    tm: term
                    x: term.cx(15 + root.gaugeCells + 1)
                    shownText: master.shown >= 0 ? master.shown + "%" : "—"
                    strong: root.isCursor(-1, "value")
                }
            }
            Item {
                visible: root.showAll
                width: parent.width
                height: term.ch
            }

            // --- one block per display ----------------------------------------------
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
                    readonly property bool current: root.isCursor(block.index, "value")

                    width: column.width

                    // A blank row between displays.
                    Item {
                        visible: block.index > 0
                        width: parent.width
                        height: term.ch
                    }

                    // The display: label, gauge, value.
                    Item {
                        width: parent.width
                        height: term.ch

                        HgRow {
                            tm: term
                            theme: root.theme
                            label: block.m.logicalId
                            current: block.current
                            active: root.activeFocus
                            dim: !block.m.enabled
                            onClicked: root.select(block.index, "value")
                        }
                        HgGauge {
                            tm: term
                            theme: root.theme
                            x: term.cx(15)
                            cells: root.gaugeCells
                            value: block.shown
                            marker: block.manual && block.target >= 0 ? block.target : -1
                            interactive: block.canWrite && block.shown >= 0
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
                        HgValue {
                            tm: term
                            x: term.cx(15 + root.gaugeCells + 1)
                            shownText: block.shown >= 0 ? block.shown + "%" : "—"
                            strong: block.current
                            color: block.manual ? root.theme.warning : root.theme.fg
                        }
                    }

                    // └ [ AUTO ] target 39% · range 1–50% · a auto
                    Item {
                        width: parent.width
                        height: term.ch

                        TermRuns {
                            id: detail

                            tm: term
                            x: term.cx(2)
                            cells: Math.max(0, root.innerCols - 2)
                            defaultColor: root.theme.textMuted
                            runs: {
                                const t = root.theme;
                                const out = [{ t: term.detailGlyph + " ", c: t.textMuted }];
                                const m = block.m;
                                if (!m.enabled) {
                                    out.push({ t: i18ndc(root.domain, "This display is excluded from automation", "[ OFF ]"), c: t.textMuted, b: true });
                                } else if (m.unreachable) {
                                    out.push({ t: i18ndc(root.domain, "Display is not answering; seconds until the next try",
                                                         "[ RETRY %1s ]", m.backoffRemainingS), c: t.warning, b: true });
                                } else if (block.unavailable) {
                                    out.push({ t: i18ndc(root.domain, "This display is not answering", "[ OFFLINE ]"), c: t.error, b: true });
                                } else if (block.manual) {
                                    out.push(Hg.flag(t, "warn", i18ndc(root.domain, "This brightness was set by hand", "MANUAL")));
                                } else {
                                    out.push(Hg.flag(t, "ok", i18ndc(root.domain, "Brightness follows the automatic curve", "AUTO")));
                                }
                                const tail = [{ t: " · ", c: t.borderInactive },
                                              { t: i18ndc(root.domain, "Brightness range limits", "range %1–%2%",
                                                          block.limits[0], block.limits[1]),
                                                c: block.open || rangeHit.containsMouse ? t.accent : t.textMuted, k: "range" }];
                                if (block.manual) {
                                    tail.push({ t: "  " });
                                    tail.push({ t: "a", c: autoHit.containsMouse ? t.accent : t.fg, b: true, k: "auto" });
                                    tail.push({ t: " " + i18ndc(root.domain, "Key hint: back to automatic brightness", "auto"),
                                                c: autoHit.containsMouse ? t.accent : t.textMuted, k: "auto2" });
                                }
                                // The target goes first when the line is too long: the way back (the key hint) stays.
                                const target = { t: " target " + block.target + "%", c: t.textMuted };
                                const used = out.concat(tail).reduce((n, r) => n + term.len(r.t), 0);
                                if (block.target >= 0 && m.enabled && !block.unavailable
                                        && used + term.len(target.t) <= Math.max(0, root.innerCols - 2)) {
                                    out.push(target);
                                }
                                return out.concat(tail);
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

                    // The range: the track, then the minimum and maximum rows.
                    Item {
                        visible: block.open
                        width: parent.width
                        height: term.ch

                        TermRange {
                            tm: term
                            theme: root.theme
                            x: term.cx(4)
                            cells: Math.max(10, root.innerCols - 4)
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

                        delegate: Item {
                            required property string modelData
                            required property int index

                            width: block.width
                            height: term.ch

                            HgRow {
                                tm: term
                                theme: root.theme
                                x: term.cx(2)
                                label: index === 0 ? i18ndc(root.domain, "Night minimum boundary", "minimum")
                                                   : i18ndc(root.domain, "Day maximum boundary", "maximum")
                                current: root.isCursor(block.index, modelData)
                                active: root.activeFocus
                                onClicked: root.select(block.index, modelData)
                            }
                        }
                    }
                }
            }
        }
    }

    /*! One label row: the cursor glyph, the label in its cell column, and the
        cursor row in inverse video while the list has focus. */
    component HgRow: Item {
        id: row

        required property TermMetrics tm
        required property Theme theme
        property string label: ""
        property bool current: false
        property bool active: false
        property bool dim: false

        signal clicked()

        width: parent ? parent.width : 0
        height: tm.ch

        MouseArea {
            anchors.fill: parent
            onClicked: row.clicked()
        }

        TermText {
            tm: row.tm
            visible: row.current
            value: row.active ? ">" : "›"
            strong: row.active
            color: row.theme.accent
        }

        TermText {
            tm: row.tm
            x: row.tm.cx(2)
            value: Term.padEnd(row.label, 12)
            strong: row.current
            color: row.current && row.active ? row.theme.bg
                : (row.dim ? row.theme.textMuted : row.theme.fg)
            inverse: row.current && row.active
            inverseColor: row.theme.accent
        }
    }

    /*! The value at the end of a gauge row, right-aligned in four cells. */
    component HgValue: TermText {
        property string shownText: ""

        value: Term.padStart(shownText, 4)
        strong: true
        color: root.theme.fg
    }
}
