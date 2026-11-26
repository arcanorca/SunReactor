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
import "../../modules/Monitors.js" as Monitors
import "../terminal"
import "../terminal/Term.js" as Term
import "Hg.js" as Hg

FocusScope {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    readonly property bool showAll: monitors.length > 1
    readonly property bool available: Monitors.hasDisplayData(client)

    TermMetrics {
        id: term

        theme: root.theme
    }
    HgCrt {
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
        interactive: root.client.isConnected
    }

    activeFocusOnTab: true
    implicitHeight: section.implicitHeight

    Keys.onUpPressed: event => ctl.up()
    Keys.onDownPressed: event => ctl.down()
    Keys.onLeftPressed: event => ctl.step(event, -1)
    Keys.onRightPressed: event => ctl.step(event, 1)
    Keys.onPressed: event => ctl.handleKey(event)

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
                      (ctl.cursor && ctl.cursor.mon >= 0 ? ctl.cursor.mon : 0) + 1, root.monitors.length),
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

                readonly property int shown: ctl.shownPct("*", root.client.globalPercent)

                HgRow {
                    tm: term
                    theme: root.theme
                    label: root.settings.allDisplaysLabel
                    current: ctl.isCursor(-1, "value")
                    active: root.activeFocus
                    onClicked: ctl.select(-1, "value")
                }
                HgGauge {
                    tm: term
                    theme: root.theme
                    x: term.cx(15)
                    cells: root.gaugeCells
                    value: master.shown
                    interactive: root.client.isConnected && root.client.globalPercent >= 0
                    fillColor: root.client.isOverrideActive ? root.theme.warning : root.theme.accent
                    onMoved: value => {
                        ctl.select(-1, "value");
                        ctl.setPct("*", value);
                    }
                    onReleased: value => {
                        ctl.setPct("*", value);
                        ctl.flush();
                    }
                }
                HgValue {
                    tm: term
                    x: term.cx(15 + root.gaugeCells + 1)
                    shownText: master.shown >= 0 ? master.shown + "%" : "—"
                    strong: ctl.isCursor(-1, "value")
                }
            }

            // --- one block per display (no blank row between them: the Brightness page budget) ---
            Repeater {
                model: root.monitors

                delegate: Column {
                    id: block

                    required property var modelData
                    required property int index

                    readonly property var m: modelData
                    readonly property bool canWrite: root.client.isConnected && Monitors.controllable(m)
                    readonly property bool hasLimits: Monitors.hasValidLimits(m)
                    readonly property bool open: hasLimits && ctl.expanded[m.logicalId] === true
                    readonly property int shown: ctl.shownPct(m.logicalId, m.percent)
                    readonly property var limits: hasLimits ? ctl.shownLimits(m) : [0, 100]
                    readonly property int target: ctl.targetNow(m.logicalId)
                    readonly property bool manual: m.hasOverride === true
                    readonly property bool unavailable: Monitors.unavailable(m)
                    readonly property bool current: ctl.isCursor(block.index, "value")

                    width: column.width

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
                            onClicked: ctl.select(block.index, "value")
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
                                ctl.select(block.index, "value");
                                ctl.setPct(block.m.logicalId, value);
                            }
                            onReleased: value => {
                                ctl.setPct(block.m.logicalId, value);
                                ctl.flush();
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
                                const notice = Monitors.monitorNotice(
                                    m, root.settings.contestedNotice, root.settings.deferredNotice);
                                if (!m.enabled) {
                                    out.push({ t: i18ndc(root.domain, "This display is excluded from automation", "[ OFF ]"), c: t.textMuted, b: true });
                                } else if (m.unreachable) {
                                    out.push({ t: i18ndc(root.domain, "Display is not answering; seconds until the next try",
                                                         "[ RETRY %1s ]", m.backoffRemainingS), c: t.warning, b: true });
                                } else if (block.unavailable) {
                                    out.push({ t: i18ndc(root.domain, "This display is not answering", "[ OFFLINE ]"), c: t.error, b: true });
                                } else if (notice.length > 0) {
                                    out.push(Hg.flag(t, "warn", notice));
                                } else if (block.manual) {
                                    out.push(Hg.flag(t, "warn", i18ndc(root.domain, "This brightness was set by hand", "MANUAL")));
                                } else {
                                    out.push(Hg.flag(t, "ok", i18ndc(root.domain, "Brightness follows the automatic curve", "AUTO")));
                                }
                                const tail = [];
                                if (block.hasLimits) {
                                    tail.push({ t: " · ", c: t.borderInactive });
                                    tail.push({ t: i18ndc(root.domain, "Brightness range limits", "range %1–%2%",
                                                          block.limits[0], block.limits[1]),
                                                c: block.open || rangeHit.containsMouse ? t.accent : t.textMuted, k: "range" });
                                }
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
                            visible: block.hasLimits && span !== null
                            x: detail.x + term.cx(span ? span[0] : 0)
                            width: term.cx(span ? span[1] : 0)
                            height: term.ch
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                ctl.toggleRange(block.m);
                                ctl.select(block.index, block.open ? "min" : "value");
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
                            onClicked: ctl.automatic(block.index)
                        }
                    }

                    // The range: the track, then the minimum and maximum rows.
                    Item {
                        visible: block.open
                        enabled: root.client.isConnected && block.hasLimits
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
                            focusedHandle: root.activeFocus && ctl.cursor && ctl.cursor.mon === block.index
                                && ctl.cursor.field !== "value" ? ctl.cursor.field : ""
                            onMoved: (minPct, maxPct) => {
                                ctl.select(block.index, minPct !== block.limits[0] ? "min" : "max");
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

                        delegate: Item {
                            id: rangeChoice

                            required property string modelData
                            required property int index

                            width: block.width
                            height: term.ch

                            HgRow {
                                tm: term
                                theme: root.theme
                                x: term.cx(2)
                                label: rangeChoice.index === 0 ? i18ndc(root.domain, "Night minimum boundary", "minimum")
                                                   : i18ndc(root.domain, "Day maximum boundary", "maximum")
                                current: ctl.isCursor(block.index, rangeChoice.modelData)
                                active: root.activeFocus
                                onClicked: ctl.select(block.index, rangeChoice.modelData)
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
