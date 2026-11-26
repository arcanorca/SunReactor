/*
 * Shared slider commit policy: 250 ms trailing debounce during pointer drags,
 * immediate flush on pointer release, coalesced debounce for keyboard nudges,
 * and suppression of no-op commits when the value matches the daemon's last
 * reported value or the user clicked without changing the value.
 *
 * Set `debounceMs: -1` for release-only pointer behaviour (e.g. C64); keyboard
 * nudges still coalesce through `nudgeDebounceMs`.
 */

pragma ComponentBehavior: Bound

import QtQuick

Item {
    id: root

    /*! Trailing debounce delay (ms) while dragging. Negative = release-only. */
    property int debounceMs: 250
    /*! Trailing debounce delay (ms) for keyboard nudges. */
    property int nudgeDebounceMs: 250
    /*! Last value reported by the daemon (-1 when unknown). */
    property int reportedValue: -1

    signal committed(int value)

    property bool _hasPending: false
    property int _pendingValue: -1
    property bool _hasLastSent: false
    property int _lastSentValue: -1

    readonly property bool pending: root._hasPending
    readonly property int pendingValue: root._pendingValue

    visible: false
    width: 0
    height: 0

    onReportedValueChanged: {
        if (!debounceTimer.running && !root._hasPending) {
            root._hasLastSent = false;
            root._lastSentValue = -1;
        }
    }

    function _baseline(): int {
        return root._hasLastSent ? root._lastSentValue : root.reportedValue;
    }

    function _flushIfChanged(): void {
        debounceTimer.stop();
        if (!root._hasPending) {
            return;
        }
        const val = root._pendingValue;
        const baseline = root._baseline();
        root._hasPending = false;
        if (val < 0 || val === baseline) {
            return;
        }
        root._hasLastSent = true;
        root._lastSentValue = val;
        root.committed(val);
    }

    /*! Schedule a value change from pointer drag/move. */
    function schedule(value): void {
        const v = Math.round(value);
        if (v < 0) {
            return;
        }
        if (v === root._baseline() && !root._hasLastSent) {
            root._hasPending = false;
            debounceTimer.stop();
            return;
        }
        root._pendingValue = v;
        root._hasPending = true;
        if (root.debounceMs >= 0) {
            debounceTimer.interval = root.debounceMs;
            debounceTimer.restart();
        }
    }

    /*! Schedule a value change from a keyboard nudge; always debounced. */
    function nudge(value): void {
        const v = Math.round(value);
        if (v < 0) {
            return;
        }
        if (v === root._baseline() && !root._hasLastSent) {
            root._hasPending = false;
            debounceTimer.stop();
            return;
        }
        root._pendingValue = v;
        root._hasPending = true;
        debounceTimer.interval = root.nudgeDebounceMs >= 0 ? root.nudgeDebounceMs : 250;
        debounceTimer.restart();
    }

    /*! Flush on pointer release or cancel. Only commits if a move occurred or
        the value differs from the baseline. */
    function release(value = undefined): void {
        debounceTimer.stop();
        if (value !== undefined && value !== null) {
            const v = Math.round(value);
            if (v >= 0 && (root._hasPending || (root._hasLastSent && v !== root._lastSentValue))) {
                root._pendingValue = v;
                root._hasPending = true;
            }
        }
        root._flushIfChanged();
        root._hasLastSent = false;
        root._lastSentValue = -1;
    }

    /*! Discard any pending or sent tracking state. */
    function reset(): void {
        debounceTimer.stop();
        root._hasPending = false;
        root._pendingValue = -1;
        root._hasLastSent = false;
        root._lastSentValue = -1;
    }

    Timer {
        id: debounceTimer

        repeat: false
        onTriggered: root._flushIfChanged()
    }
}
