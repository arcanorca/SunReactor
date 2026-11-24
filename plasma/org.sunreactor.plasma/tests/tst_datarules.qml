/*
 * The data rules the modules and skins share (qmltestrunner, offscreen): the
 * day-curve lookups (modules/DayCurve.js), what a display's state means for its
 * controls (modules/Monitors.js), and the terminal family's Displays control
 * (skins/terminal/DisplaysControl.qml: cursor, pending values, keys). The client
 * is an offline SunReactorClient; nothing reaches a daemon.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtTest
import org.sunreactor.plasma

import "../package/contents/ui/modules"
import "../package/contents/ui/modules/DayCurve.js" as DayCurve
import "../package/contents/ui/modules/Monitors.js" as Monitors
import "../package/contents/ui/skins/terminal"
import "../package/contents/ui/skins/hackergreen/Hg.js" as Hg
import "../package/contents/ui/components/weathericons"
import "../package/contents/ui/engine"

Item {
    id: stage

    width: 200
    height: 200

    readonly property var samples: [
        { t: 100, pct: 10, pctNoWeather: 20 },
        { t: 200, pct: 50, pctNoWeather: 60 },
        { t: 300, pct: 30, pctNoWeather: 30 },
    ]
    readonly property var dayCurve: ({
        monitors: [
            { monitorId: "mi-monitor", samples: stage.samples },
            { monitorId: "len-p24h-20", samples: [{ t: 0, pct: 5, pctNoWeather: 5 }] },
        ],
    })
    readonly property var monitors: [
        { logicalId: "mi-monitor", enabled: true, unreachable: false, topology: "present", percent: 40, minPct: 1, maxPct: 50 },
        { logicalId: "len-p24h-20", enabled: true, unreachable: false, topology: "present", percent: 60, minPct: 5, maxPct: 60 },
    ]

    SunReactorClient {
        id: client
    }

    ModuleSettings {
        id: settings
    }

    Item {
        id: scope

        focus: true
        activeFocusOnTab: true
    }

    DisplaysControl {
        id: control

        client: client
        settings: settings
        scope: scope
        monitors: stage.monitors
        showAll: true
        jumpKeys: true
    }

    TestCase {
        name: "DayCurve"

        function test_valueAt() {
            compare(DayCurve.valueAt(stage.samples, 100, "pct"), 10, "first sample");
            compare(DayCurve.valueAt(stage.samples, 150, "pct"), 30, "midway between two samples");
            compare(DayCurve.valueAt(stage.samples, 250, "pctNoWeather"), 45, "another field, interpolated");
            compare(DayCurve.valueAt(stage.samples, 300, "pct"), 30, "last sample");
            compare(DayCurve.valueAt(stage.samples, 99, "pct"), -1, "before the day");
            compare(DayCurve.valueAt(stage.samples, 301, "pct"), -1, "after the day");
            compare(DayCurve.valueAt([], 100, "pct"), -1, "no samples");
            compare(DayCurve.valueAt(null, 100, "pct"), -1, "no list");
        }

        function test_curveFor_and_targetAt() {
            compare(DayCurve.curveFor(stage.dayCurve, "mi-monitor").monitorId, "mi-monitor");
            compare(DayCurve.curveFor(stage.dayCurve, "nope"), null);
            compare(DayCurve.curveFor(null, "mi-monitor"), null);
            compare(DayCurve.targetAt(stage.dayCurve, "mi-monitor", 150), 30);
            compare(DayCurve.targetAt(stage.dayCurve, "nope", 150), -1);
            compare(DayCurve.targetAt(stage.dayCurve, "len-p24h-20", 0), 5, "a single sample answers its own time");
        }

        function test_shortName_and_appliedFor() {
            compare(DayCurve.shortName("len-p24h-20"), "LEN");
            compare(DayCurve.shortName("mi-monitor"), "MI");
            compare(DayCurve.shortName("samsung_odyssey"), "SAMS");
            compare(DayCurve.shortName(""), "");
            const list = [{ logicalId: "a", appliedPercent: 33 }, { logicalId: "b", appliedPercent: -1 }];
            compare(DayCurve.appliedFor(list, "a"), 33);
            compare(DayCurve.appliedFor(list, "b"), -1);
            compare(DayCurve.appliedFor(list, "c"), -1, "not listed");
            compare(DayCurve.appliedFor([], "a"), -1);
        }
    }

    TestCase {
        name: "Monitors"

        function test_unavailable_and_controllable() {
            verify(!Monitors.unavailable({ unreachable: false, topology: "present" }));
            verify(!Monitors.unavailable({ topology: "present" }), "unreachable missing counts as reachable");
            verify(Monitors.unavailable({ unreachable: true, topology: "present" }));
            verify(Monitors.unavailable({ unreachable: false, topology: "temporarily_unavailable" }));
            verify(Monitors.controllable({ enabled: true, unreachable: false, topology: "present" }));
            verify(!Monitors.controllable({ enabled: false, unreachable: false, topology: "present" }));
            verify(!Monitors.controllable({ enabled: true, unreachable: true, topology: "present" }));
            verify(!Monitors.controllable({ enabled: true, topology: "temporarily_unavailable" }));
        }

        function test_rowsWithMaster() {
            const single = [{ logicalId: "eDP-1", percent: 50, enabled: true }];
            const singleRows = Monitors.rowsWithMaster(single, 50, false, "All displays");
            compare(singleRows.length, 1);
            compare(singleRows[0].logicalId, "eDP-1");

            const multiRows = Monitors.rowsWithMaster(stage.monitors, 45, true, "All displays");
            compare(multiRows.length, 3);
            verify(multiRows[0].master === true);
            compare(multiRows[0].logicalId, "All displays");
            compare(multiRows[0].percent, 45);
            verify(multiRows[0].hasOverride === true);
            compare(multiRows[1].logicalId, "mi-monitor");
            compare(multiRows[2].logicalId, "len-p24h-20");
        }
    }

    TestCase {
        name: "DisplaysControl"
        when: windowShown

        function init() {
            control.cursorIndex = 0;
            control.expanded = ({});
            control.pendingPct = ({});
            control.pendingLimits = ({});
        }

        function key(code, modifiers) {
            const event = { key: code, modifiers: modifiers || 0, accepted: false };
            control.handleKey(event);
            return event.accepted;
        }

        function test_targets_follow_the_open_ranges() {
            compare(control.targets.length, 3, "All displays and two displays");
            compare(control.targets[0].mon, -1);
            control.toggleRange(stage.monitors[0]);
            compare(control.targets.length, 5, "an open range adds its two boundary fields");
            compare(control.targets[2].field, "min");
            compare(control.targets[3].field, "max");
            control.toggleRange(stage.monitors[0]);
            compare(control.targets.length, 3);
        }

        function test_cursor_and_select() {
            verify(control.isCursor(-1, "value"));
            control.select(1, "value");
            compare(control.cursorIndex, 2);
            verify(control.isCursor(1, "value"));
            control.cursorIndex = 99;
            compare(control.cursor.mon, 1, "the cursor stays on the last control");
        }

        function test_down_and_up_move_the_cursor() {
            control.down();
            compare(control.cursorIndex, 1);
            control.down();
            compare(control.cursorIndex, 2);
            control.up();
            compare(control.cursorIndex, 1);
        }

        function test_arrows_step_and_shift_steps_ten() {
            control.select(0, "value");
            control.step({ modifiers: 0 }, 1);
            compare(control.pendingPct["mi-monitor"], 41);
            control.step({ modifiers: Qt.ShiftModifier }, -1);
            compare(control.pendingPct["mi-monitor"], 31);
            verify(key(Qt.Key_PageUp));
            compare(control.pendingPct["mi-monitor"], 41);
            verify(key(Qt.Key_PageDown));
            compare(control.pendingPct["mi-monitor"], 31);
            control.flush();
        }

        function test_home_and_end_use_the_range_when_enabled() {
            control.select(0, "value");
            verify(key(Qt.Key_End));
            compare(control.pendingPct["mi-monitor"], 50, "the top of the display's range");
            verify(key(Qt.Key_Home));
            compare(control.pendingPct["mi-monitor"], 1, "the bottom of the display's range");
            control.jumpKeys = false;
            verify(!key(Qt.Key_Home), "not handled without jumpKeys");
            control.jumpKeys = true;
            control.flush();
        }

        function test_range_fields_move_the_limits() {
            control.toggleRange(stage.monitors[0]);
            control.select(0, "min");
            control.step({ modifiers: 0 }, 1);
            compare(control.pendingLimits["mi-monitor"][0], 2);
            control.select(0, "max");
            control.step({ modifiers: 0 }, -1);
            compare(control.pendingLimits["mi-monitor"][1], 49);
            control.flush();
        }

        function test_enter_opens_and_escape_closes_the_range() {
            control.select(1, "value");
            verify(key(Qt.Key_Return));
            verify(control.expanded["len-p24h-20"] === true);
            control.select(1, "min");
            verify(key(Qt.Key_Escape));
            verify(control.expanded["len-p24h-20"] === false);
            verify(control.isCursor(1, "value"), "the cursor returns to the value");
            verify(!key(Qt.Key_Escape), "Escape with nothing open is left to the popup");
        }

        function test_unknown_keys_are_not_accepted() {
            verify(!key(Qt.Key_X));
        }

        function test_pending_value_shows_until_reported() {
            compare(control.shownPct("mi-monitor", 40), 40);
            control.setPct("mi-monitor", 77);
            compare(control.shownPct("mi-monitor", 40), 77);
            control.setPct("mi-monitor", 400);
            compare(control.shownPct("mi-monitor", 40), 100, "clamped");
            control.flush();
        }
    }

    Theme { id: iconTestTheme }

    TerminalIcon {
        id: termIconTest
        condition: "clear"
        night: false
        theme: iconTestTheme
        size: 32
    }

    TestCase {
        name: "WeatherIcons"

        readonly property var allTokens: [
            "clear", "night_clear", "partly_cloudy", "night_partly_cloudy",
            "cloudy", "drizzle", "rain", "heavy_rain", "thunderstorm",
            "snow", "mist", "fog", "atmospheric", "unknown"
        ]

        function test_hgArtUniformDimensions() {
            for (const key of allTokens) {
                verify(Hg.ART.hasOwnProperty(key), "Hg.ART has " + key);
                const rows = Hg.ART[key];
                compare(rows.length, 4, key + " has 4 rows");
                for (let i = 0; i < rows.length; ++i) {
                    compare(rows[i].length, 9, key + " row " + i + " is 9 cells wide");
                }
            }
        }

        function test_hgArtCompactUniformDimensions() {
            for (const key of allTokens) {
                verify(Hg.ART_COMPACT.hasOwnProperty(key), "ART_COMPACT has " + key);
                const rows = Hg.ART_COMPACT[key];
                compare(rows.length, 3, key + " compact has 3 rows");
                for (let i = 0; i < rows.length; ++i) {
                    compare(rows[i].length, 7, key + " compact row " + i + " is 7 cells wide");
                }
            }
            compare(Hg.artCompact("partly_cloudy", true), Hg.ART_COMPACT.night_partly_cloudy);
            compare(Hg.artCompact("bogus", false), Hg.ART_COMPACT.unknown);
        }

        function test_hgArtNightRouting() {
            compare(Hg.art("clear", false), Hg.ART.clear);
            compare(Hg.art("clear", true), Hg.ART.night_clear);
            compare(Hg.art("partly_cloudy", false), Hg.ART.partly_cloudy);
            compare(Hg.art("partly_cloudy", true), Hg.ART.night_partly_cloudy);
            compare(Hg.art("", false), Hg.ART.unknown);
            compare(Hg.art("bogus", true), Hg.ART.unknown);
        }

        function test_terminalIconGridAndTokens() {
            compare(termIconTest.gridCells, 16);
            compare(termIconTest.artPx, 2);
            for (const key of allTokens) {
                verify(termIconTest.artTable16.hasOwnProperty(key), "artTable16 has " + key);
                const g16 = termIconTest.artTable16[key];
                compare(g16.length, 16, key + " 16x16 rows");
                for (let y = 0; y < g16.length; ++y) {
                    compare(g16[y].length, 16, key + " 16x16 row " + y);
                }
                verify(termIconTest.artTable11.hasOwnProperty(key), "artTable11 has " + key);
                const g11 = termIconTest.artTable11[key];
                compare(g11.length, 11, key + " 11x11 rows");
                for (let y = 0; y < g11.length; ++y) {
                    compare(g11[y].length, 11, key + " 11x11 row " + y);
                }
                verify(termIconTest.artTable6.hasOwnProperty(key), "artTable6 has " + key);
                const g6 = termIconTest.artTable6[key];
                compare(g6.length, 6, key + " 6x6 rows");
                for (let y = 0; y < g6.length; ++y) {
                    compare(g6[y].length, 6, key + " 6x6 row " + y);
                }
            }
        }

        function test_monitorExpandedMapAndExternalHoldHelpers() {
            var map = ({});
            verify(!Monitors.isExpanded(map, "mi-0"));
            verify(!Monitors.isExpanded(null, "mi-0"));

            map = Monitors.toggleExpanded(map, "mi-0");
            verify(Monitors.isExpanded(map, "mi-0"));
            verify(!Monitors.isExpanded(map, "len-1"));

            // Toggling another id preserves existing expanded entries.
            var map2 = Monitors.toggleExpanded(map, "len-1");
            verify(Monitors.isExpanded(map2, "mi-0"));
            verify(Monitors.isExpanded(map2, "len-1"));

            // Original map remains immutable.
            verify(!Monitors.isExpanded(map, "len-1"));

            var map3 = Monitors.toggleExpanded(map2, "mi-0");
            verify(!Monitors.isExpanded(map3, "mi-0"));
            verify(Monitors.isExpanded(map3, "len-1"));

            verify(!Monitors.hasExternalHold(null));
            verify(!Monitors.hasExternalHold({ externalHold: { remainingS: 0 } }));
            compare(Monitors.externalHoldSeconds({ externalHold: { remainingS: 95 } }), 95);
            verify(Monitors.hasExternalHold({ externalHold: { remainingS: 95 } }));
            compare(Monitors.externalHoldAge(0), "<1m");
            compare(Monitors.externalHoldAge(59), "<1m");
            compare(Monitors.externalHoldAge(60), "1m");
            compare(Monitors.externalHoldAge(61), "2m");
        }
    }
}
