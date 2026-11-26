/*
 * Shared slider commit policy (qmltestrunner, offscreen): 250 ms trailing
 * debounce while dragging, immediate flush on release, coalesced keyboard
 * nudges, no-op suppression when unchanged from reported value, and C64
 * release-only mode (debounceMs: -1).
 */

import QtQuick
import QtTest

import "../package/contents/ui/components"

Item {
    id: stage

    width: 100
    height: 100

    CommitThrottle {
        id: throttle

        reportedValue: 40
    }

    CommitThrottle {
        id: releaseOnly

        debounceMs: -1
        reportedValue: 40
    }

    SignalSpy {
        id: commits

        target: throttle
        signalName: "committed"
    }

    SignalSpy {
        id: releaseOnlyCommits

        target: releaseOnly
        signalName: "committed"
    }

    TestCase {
        name: "CommitThrottle"
        when: windowShown

        function init() {
            throttle.debounceMs = 250;
            throttle.nudgeDebounceMs = 250;
            throttle.reportedValue = 40;
            throttle.reset();
            releaseOnly.debounceMs = -1;
            releaseOnly.nudgeDebounceMs = 250;
            releaseOnly.reportedValue = 40;
            releaseOnly.reset();
            commits.clear();
            releaseOnlyCommits.clear();
        }

        function test_debounce_while_dragging() {
            throttle.schedule(50);
            throttle.schedule(60);
            compare(commits.count, 0, "dragging does not commit immediately");
            verify(throttle.pending);
            compare(throttle.pendingValue, 60);

            tryCompare(commits, "count", 1, 1000);
            compare(commits.signalArguments[0][0], 60);
            verify(!throttle.pending);
        }

        function test_release_flushes_immediately_and_clears_pending() {
            throttle.schedule(55);
            verify(throttle.pending);
            compare(commits.count, 0);

            throttle.release(70);
            compare(commits.count, 1);
            compare(commits.signalArguments[0][0], 70);
            verify(!throttle.pending);

            wait(320);
            compare(commits.count, 1, "no duplicate timer commit after release");
        }

        function test_no_change_click_suppresses_commit() {
            throttle.schedule(40);
            verify(!throttle.pending);
            throttle.release(40);
            compare(commits.count, 0, "click without move never commits");

            // Drag away then back to the reported value before releasing.
            throttle.schedule(62);
            verify(throttle.pending);
            throttle.schedule(40);
            throttle.release(40);
            wait(320);
            compare(commits.count, 0, "returning to reported value before commit sends nothing");
        }

        function test_drag_commit_then_return_to_reported_value() {
            throttle.schedule(60);
            tryCompare(commits, "count", 1, 1000);
            compare(commits.signalArguments[0][0], 60);

            // Releasing at the already-committed value does not re-send 60.
            throttle.release(60);
            compare(commits.count, 1);

            // Moving back to the still-unchanged reportedValue (40) after sending 60 must send 40.
            throttle.schedule(60);
            tryCompare(commits, "count", 2, 1000);
            throttle.schedule(40);
            throttle.release(40);
            compare(commits.count, 3);
            compare(commits.signalArguments[2][0], 40);
        }

        function test_keyboard_nudge_coalesces() {
            throttle.nudge(41);
            throttle.nudge(42);
            throttle.nudge(43);
            compare(commits.count, 0, "rapid keyboard nudges coalesce");
            verify(throttle.pending);
            tryCompare(commits, "count", 1, 1000);
            compare(commits.signalArguments[0][0], 43);
        }

        function test_c64_release_only_mode() {
            releaseOnly.schedule(55);
            verify(releaseOnly.pending);
            wait(350);
            compare(releaseOnlyCommits.count, 0, "debounceMs < 0 does not emit while dragging");
            verify(releaseOnly.pending);

            releaseOnly.release(65);
            compare(releaseOnlyCommits.count, 1);
            compare(releaseOnlyCommits.signalArguments[0][0], 65);
            verify(!releaseOnly.pending);

            // No-change click in C64 mode is still suppressed.
            releaseOnly.schedule(40);
            releaseOnly.release(40);
            compare(releaseOnlyCommits.count, 1);

            // Keyboard nudges still debounce and commit in C64 mode.
            releaseOnly.nudge(70);
            tryCompare(releaseOnlyCommits, "count", 2, 1000);
            compare(releaseOnlyCommits.signalArguments[1][0], 70);
        }
    }
}
