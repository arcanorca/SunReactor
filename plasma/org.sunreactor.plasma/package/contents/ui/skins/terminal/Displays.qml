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
import "../../modules/Monitors.js" as Monitors
import "../../Format.js" as Format
import "Term.js" as Term

FocusScope {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    /*! The master control only from three displays: with two it repeats the two rows. */
    readonly property bool showAll: monitors.length > 2
    readonly property bool available: Monitors.hasDisplayData(client)
    /*! Rows per control: name and value, the bar (two rows), the state line. */
    readonly property int controlRows: 4
    /*! An open range editor: its track and the two boundary fields. */
    readonly property int rangeRows: 3

    TermMetrics {
        id: term
        theme: root.theme
    }

    // --- cursor, pending values and writes: shared with the other terminal skin ----
    DisplaysControl {
        id: ctl

        client: root.client
        settings: root.settings
        scope: root
        monitors: root.monitors
        showAll: root.showAll
        jumpKeys: true
        interactive: root.client.isConnected
    }

    /*! The controls, top to bottom: mon -1 is "All displays", else an index. */
    readonly property var rows: {
        const out = [];
        if (showAll) {
            out.push({ mon: -1 });
        }
        monitors.forEach((m, i) => out.push({ mon: i }));
        return out;
    }
    readonly property int openRanges: monitors.filter(m => ctl.expanded[m.logicalId] === true).length
    readonly property int contentRows: Math.max(1, rows.length * controlRows + openRanges * rangeRows)

    activeFocusOnTab: true
    implicitHeight: frame.implicitHeight

    Keys.onUpPressed: event => ctl.up()
    Keys.onDownPressed: event => ctl.down()
    Keys.onLeftPressed: event => ctl.step(event, -1)
    Keys.onRightPressed: event => ctl.step(event, 1)
    Keys.onPressed: event => ctl.handleKey(event)

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
            const mon = ctl.cursor && ctl.cursor.mon >= 0 ? ctl.cursor.mon : 0;
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
                    readonly property bool canWrite: isAll ? (root.client.isConnected && root.client.globalPercent >= 0)
                                                             : (root.client.isConnected && Monitors.controllable(m))
                    readonly property int shown: isAll ? ctl.shownPct("*", root.client.globalPercent)
                                                       : ctl.shownPct(m.logicalId, m.percent)
                    readonly property bool hasLimits: !isAll && Monitors.hasValidLimits(m)
                    readonly property var limits: isAll || !hasLimits ? [0, 100] : ctl.shownLimits(m)
                    readonly property int target: isAll ? -1 : ctl.targetNow(m.logicalId)
                    readonly property bool manual: isAll ? root.client.isOverrideActive : m.hasOverride === true
                    readonly property bool open: hasLimits && ctl.expanded[m.logicalId] === true
                    readonly property bool unavailable: !isAll && Monitors.unavailable(m)
                    readonly property bool focusedHere: root.activeFocus && ctl.isCursor(mon, "value")
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
                            visible: ctl.isCursor(block.mon, "value")
                            value: root.activeFocus ? "❯" : "›"
                            strong: root.activeFocus
                            color: block.focusedHere ? root.theme.onColor(root.theme.accent)
                                : (root.activeFocus ? root.theme.accent : root.theme.textMuted)
                        }
                        TermText {
                            tm: term
                            x: term.cx(2)
                            value: block.isAll ? root.settings.allDisplaysLabel : block.m.logicalId
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
                            onClicked: ctl.select(block.mon, "value")
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
                            ctl.select(block.mon, "value");
                            ctl.setPct(block.key, value);
                        }
                        onReleased: value => {
                            ctl.setPct(block.key, value);
                            ctl.flush();
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
                                const notice = Monitors.monitorNotice(
                                    m, root.settings.contestedNotice, root.settings.deferredNotice);
                                if (!m.enabled) {
                                    out.push({ t: "○ " + i18ndc(root.domain, "This display is excluded from automation", "Off"), c: t.textMuted });
                                } else if (m.unreachable) {
                                    out.push({ t: "▲ " + i18ndc(root.domain, "Display is not answering; seconds until the next try",
                                                                "Retrying in %1s", m.backoffRemainingS), c: t.warning });
                                } else if (block.unavailable) {
                                    out.push({ t: "▲ " + i18ndc(root.domain, "This display is not answering", "Unavailable"), c: t.warning });
                                } else if (notice.length > 0) {
                                    out.push({ t: "▲ " + notice, c: t.warning, b: true });
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
                                    if (block.hasLimits) {
                                        out.push({ t: " · ", c: t.borderInactive });
                                        out.push({ t: i18ndc(root.domain, "Brightness range limits", "range %1–%2%",
                                                             block.limits[0], block.limits[1]),
                                                   c: block.open || rangeHit.containsMouse ? t.accent : t.textMuted, k: "range" });
                                    }
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
                            visible: block.hasLimits && span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                ctl.toggleRange(block.m);
                                ctl.select(block.mon, block.open ? "min" : "value");
                            }
                        }
                        MouseArea {
                            id: autoHit

                            readonly property var span: detail.span("auto")
                            visible: root.client.isConnected && span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] + 5 : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: ctl.automatic(block.mon)
                        }
                    }

                    // Range editor (displays only).
                    Item {
                        visible: block.open
                        enabled: root.client.isConnected && block.hasLimits
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
                            focusedHandle: root.activeFocus && ctl.cursor && ctl.cursor.mon === block.mon
                                && ctl.cursor.field !== "value" ? ctl.cursor.field : ""
                            onMoved: (minPct, maxPct) => {
                                ctl.select(block.mon, minPct !== block.limits[0] ? "min" : "max");
                                ctl.setLimits(block.m, minPct, maxPct);
                            }
                            onReleased: (minPct, maxPct) => {
                                ctl.setLimits(block.m, minPct, maxPct);
                                ctl.flush();
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
                            current: ctl.isCursor(block.mon, modelData)
                            active: root.activeFocus
                            labelColor: root.theme.fg
                            onClicked: ctl.select(block.mon, modelData)
                            onStepped: direction => {
                                ctl.select(block.mon, modelData);
                                ctl.adjust(direction);
                            }
                        }
                    }
                }
            }
        }
    }
}
