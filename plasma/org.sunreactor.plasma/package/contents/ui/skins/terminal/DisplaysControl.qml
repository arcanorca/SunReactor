/*
 * The keyboard-and-write logic of the terminal family's Displays list (terminal
 * and hackergreen draw it differently, they act the same): the cursor over the
 * controls, the values and ranges asked for but not yet reported back, the
 * throttled writes to the daemon, and the keys.
 *
 * The skin owns the focus scope and the drawing; it hands the scope in and routes
 * its key events here:
 *
 *   Keys.onUpPressed: ctl.up()           Keys.onDownPressed: ctl.down()
 *   Keys.onLeftPressed: event => ctl.step(event, -1)   (and Right, +1)
 *   Keys.onPressed: event => ctl.handleKey(event)
 *
 * A "control" is one cursor stop: { mon, field } with mon -1 for "All displays"
 * (shown only when showAll), else an index into monitors, and field "value", or
 * "min" / "max" for an open range editor.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../components"
import "../../modules"
import "../../modules/Monitors.js" as Monitors

Item {
    id: root

    required property SunReactorClient client
    required property ModuleSettings settings
    /*! The skin's focus scope: it takes focus on a click and hands it on at the list's ends. */
    required property Item scope
    required property var monitors
    required property bool showAll
    /*! Home / End jump to the ends of the cursor's range. */
    property bool jumpKeys: false
    /*! False when stale/disconnected so sliders and keys cannot queue writes. */
    property bool interactive: true

    /*! logicalId -> true while that display's range is open. */
    property var expanded: ({})
    /*! Values asked for that the daemon has not reported back yet: logicalId (or "*" for all
        displays) -> percent. */
    property var pendingPct: ({})
    /*! logicalId -> [min, max]. */
    property var pendingLimits: ({})
    property var _dirtyPct: ({})
    property var _dirtyLimits: ({})
    property var _sentPct: ({})
    property var _sentLimits: ({})
    property int _throttleSeq: 0
    property int cursorIndex: 0

    readonly property var targets: {
        const out = [];
        if (showAll) {
            out.push({ mon: -1, field: "value" });
        }
        monitors.forEach((m, i) => {
            out.push({ mon: i, field: "value" });
            if (expanded[m.logicalId] && Monitors.hasValidLimits(m)) {
                out.push({ mon: i, field: "min" });
                out.push({ mon: i, field: "max" });
            }
        });
        return out;
    }
    readonly property var cursor: targets.length > 0 ? targets[Math.max(0, Math.min(cursorIndex, targets.length - 1))] : null

    visible: false

    function isCursor(mon, field) {
        return cursor !== null && cursor.mon === mon && cursor.field === field;
    }
    function select(mon, field) {
        const i = targets.findIndex(t => t.mon === mon && t.field === field);
        if (i >= 0) {
            cursorIndex = i;
        }
        scope.forceActiveFocus(Qt.MouseFocusReason);
    }

    function shownPct(key, reported) {
        return pendingPct[key] !== undefined ? pendingPct[key] : reported;
    }
    function shownLimits(m) {
        return pendingLimits[m.logicalId] !== undefined ? pendingLimits[m.logicalId] : [m.minPct, m.maxPct];
    }
    function _reportedPct(key) {
        if (key === "*") {
            return Math.round(client.globalPercent);
        }
        for (let i = 0; i < monitors.length; ++i) {
            if (monitors[i].logicalId === key) {
                return monitors[i].percent !== undefined ? Math.round(monitors[i].percent) : -1;
            }
        }
        return -1;
    }
    function _hasDirty() {
        for (const k in _dirtyPct) {
            if (Object.prototype.hasOwnProperty.call(_dirtyPct, k)) {
                return true;
            }
        }
        for (const k in _dirtyLimits) {
            if (Object.prototype.hasOwnProperty.call(_dirtyLimits, k)) {
                return true;
            }
        }
        return false;
    }
    function _kickThrottle(fromKey) {
        if (!_hasDirty()) {
            sendThrottle.reset();
            return;
        }
        _throttleSeq += 1;
        if (fromKey) {
            sendThrottle.nudge(_throttleSeq);
        } else {
            sendThrottle.schedule(_throttleSeq);
        }
    }
    function setPct(key, value, fromKey) {
        if (!interactive) {
            return;
        }
        const clamped = Math.max(0, Math.min(100, Math.round(value)));
        const baseline = _sentPct[key] !== undefined ? _sentPct[key] : _reportedPct(key);
        const nextPending = Object.assign({}, pendingPct);
        const nextDirty = Object.assign({}, _dirtyPct);
        if (clamped === baseline) {
            delete nextDirty[key];
            if (_sentPct[key] === undefined) {
                delete nextPending[key];
            }
            pendingPct = nextPending;
            _dirtyPct = nextDirty;
            _kickThrottle(fromKey);
            return;
        }
        nextPending[key] = clamped;
        nextDirty[key] = clamped;
        pendingPct = nextPending;
        _dirtyPct = nextDirty;
        _kickThrottle(fromKey);
        settleTimer.restart();
    }
    function setLimits(m, minPct, maxPct, fromKey) {
        if (!interactive || !Monitors.hasValidLimits(m)) {
            return;
        }
        const lo = Math.max(0, Math.min(100, Math.round(minPct)));
        const hi = Math.max(lo, Math.min(100, Math.round(maxPct)));
        const key = m.logicalId;
        const base = _sentLimits[key] !== undefined ? _sentLimits[key] : [m.minPct, m.maxPct];
        const nextPending = Object.assign({}, pendingLimits);
        const nextDirty = Object.assign({}, _dirtyLimits);
        if (lo === base[0] && hi === base[1]) {
            delete nextDirty[key];
            if (_sentLimits[key] === undefined) {
                delete nextPending[key];
            }
            pendingLimits = nextPending;
            _dirtyLimits = nextDirty;
            _kickThrottle(fromKey);
            return;
        }
        nextPending[key] = [lo, hi];
        nextDirty[key] = [lo, hi];
        pendingLimits = nextPending;
        _dirtyLimits = nextDirty;
        _kickThrottle(fromKey);
        settleTimer.restart();
    }
    function flush() {
        sendThrottle.reset();
        if (!interactive) {
            return;
        }
        const sentP = Object.assign({}, _sentPct);
        for (const key in _dirtyPct) {
            if (!Object.prototype.hasOwnProperty.call(_dirtyPct, key)) {
                continue;
            }
            const val = _dirtyPct[key];
            sentP[key] = val;
            if (key === "*") {
                client.setGlobalOverride(val, settings.overrideMinutes);
            } else {
                client.setMonitorOverride(key, val, settings.overrideMinutes);
            }
        }
        const sentL = Object.assign({}, _sentLimits);
        for (const key in _dirtyLimits) {
            if (!Object.prototype.hasOwnProperty.call(_dirtyLimits, key)) {
                continue;
            }
            const pair = _dirtyLimits[key];
            sentL[key] = pair;
            client.setMonitorLimits(key, pair[0], pair[1]);
        }
        _sentPct = sentP;
        _sentLimits = sentL;
        _dirtyPct = ({});
        _dirtyLimits = ({});
    }
    function toggleRange(m) {
        if (!Monitors.hasValidLimits(m)) {
            return;
        }
        const next = Object.assign({}, expanded);
        next[m.logicalId] = !next[m.logicalId];
        expanded = next;
    }
    function automatic(mon) {
        if (!interactive) {
            return;
        }
        if (mon < 0) {
            client.clearAllOverrides();
        } else {
            client.clearMonitorOverride(monitors[mon].logicalId);
        }
        pendingPct = ({});
        _dirtyPct = ({});
        _sentPct = ({});
    }
    /*! Writes the value of the cursor's bar (or the range boundary it is on). */
    function adjust(delta) {
        if (!cursor || !interactive) {
            return;
        }
        if (cursor.mon < 0) {
            if (client.globalPercent >= 0) {
                setPct("*", shownPct("*", client.globalPercent) + delta, true);
            }
            return;
        }
        const m = monitors[cursor.mon];
        if (cursor.field === "value") {
            if (Monitors.controllable(m) && m.percent >= 0) {
                setPct(m.logicalId, shownPct(m.logicalId, m.percent) + delta, true);
            }
        } else if (Monitors.hasValidLimits(m)) {
            const lim = shownLimits(m);
            if (cursor.field === "min") {
                setLimits(m, Math.min(lim[0] + delta, lim[1]), lim[1], true);
            } else {
                setLimits(m, lim[0], Math.max(lim[1] + delta, lim[0]), true);
            }
        }
    }
    /*! Home / End: the bottom or top of the bar's range (0 and 100 for all). */
    function jump(top) {
        if (!cursor || cursor.field !== "value" || !interactive) {
            return;
        }
        if (cursor.mon < 0) {
            if (client.globalPercent >= 0) {
                setPct("*", top ? 100 : 0, true);
            }
            return;
        }
        const m = monitors[cursor.mon];
        if (Monitors.controllable(m) && m.percent >= 0) {
            const lim = Monitors.hasValidLimits(m) ? shownLimits(m) : [0, 100];
            setPct(m.logicalId, top ? lim[1] : lim[0], true);
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

    // --- keys ----------------------------------------------------------------------
    /*! Up: the previous control; from the first one, focus moves on to the item before the list. */
    function up() {
        if (cursorIndex > 0) {
            cursorIndex = Math.min(cursorIndex, targets.length - 1) - 1;
        } else {
            const previous = scope.nextItemInFocusChain(false);
            if (previous) {
                previous.forceActiveFocus(Qt.BacktabFocusReason);
            }
        }
    }
    /*! Down: the next control; from the last one, focus moves on to the item after the list. */
    function down() {
        if (cursorIndex < targets.length - 1) {
            ++cursorIndex;
        } else {
            const next = scope.nextItemInFocusChain(true);
            if (next) {
                next.forceActiveFocus(Qt.TabFocusReason);
            }
        }
    }
    /*! Left / Right: one step, ten with Shift. */
    function step(event, direction) {
        adjust(((event.modifiers & Qt.ShiftModifier) ? 10 : 1) * direction);
    }
    /*! The remaining keys; accepts the event only when it acted on it. */
    function handleKey(event) {
        switch (event.key) {
        case Qt.Key_PageUp:
            adjust(10);
            break;
        case Qt.Key_PageDown:
            adjust(-10);
            break;
        case Qt.Key_Home:
            if (!jumpKeys) {
                return;
            }
            jump(false);
            break;
        case Qt.Key_End:
            if (!jumpKeys) {
                return;
            }
            jump(true);
            break;
        case Qt.Key_Return:
        case Qt.Key_Enter:
        case Qt.Key_Space:
            if (cursor && cursor.mon >= 0) {
                toggleRange(monitors[cursor.mon]);
            }
            break;
        case Qt.Key_A:
        case Qt.Key_Backspace:
        case Qt.Key_Delete:
            if (cursor) {
                automatic(cursor.mon);
            }
            break;
        case Qt.Key_Escape:
            if (cursor && cursor.mon >= 0 && expanded[monitors[cursor.mon].logicalId]) {
                toggleRange(monitors[cursor.mon]);
                cursorIndex = targets.findIndex(t => t.mon === cursor.mon && t.field === "value");
            } else {
                return;
            }
            break;
        default:
            return;
        }
        event.accepted = true;
    }

    CommitThrottle {
        id: sendThrottle

        reportedValue: -1
        onCommitted: root.flush()
    }
    // After the last change, give the daemon time to report it back.
    Timer {
        id: settleTimer

        interval: 2500
        onTriggered: {
            root.pendingPct = ({});
            root.pendingLimits = ({});
            root._dirtyPct = ({});
            root._dirtyLimits = ({});
            root._sentPct = ({});
            root._sentLimits = ({});
        }
    }
}
