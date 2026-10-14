/*
 * The TUI's workspace tabs, used as a switcher: numbered labels on one row,
 * a hairline rule under them and a heavy accent segment under the current
 * one (chrome.rs render_tabs). Click, or the number keys / ←→ while focused.
 *
 *     1 Mi   2 Len   3 All
 *     ───────━━━━━──────────────────
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

FocusScope {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property var labels: []
    property int current: 0
    property int cells: tm.cols(width)

    signal activated(int index)

    readonly property int gap: 3
    readonly property var starts: {
        const out = [];
        let at = 1;
        for (let i = 0; i < labels.length; ++i) {
            out.push(at);
            at += tm.len(entry(i)) + gap;
        }
        return out;
    }
    function entry(i) {
        return (i + 1) + " " + labels[i];
    }

    implicitHeight: tm.cy(2)
    activeFocusOnTab: labels.length > 1

    Keys.onLeftPressed: root.activated((current + labels.length - 1) % labels.length)
    Keys.onRightPressed: root.activated((current + 1) % labels.length)
    Keys.onPressed: event => {
        const n = parseInt(event.text, 10);
        if (n >= 1 && n <= labels.length) {
            root.activated(n - 1);
            event.accepted = true;
        }
    }

    TermText {
        tm: root.tm
        visible: root.activeFocus
        value: "❯"
        strong: true
        color: root.theme.accent
    }

    Repeater {
        model: root.labels.length

        delegate: TermText {
            id: tab

            required property int index
            readonly property bool isCurrent: index === root.current

            tm: root.tm
            x: root.tm.cx(root.starts[index])
            value: root.entry(index)
            cells: Math.max(0, root.cells - root.starts[index])
            strong: isCurrent
            color: isCurrent ? (root.activeFocus ? root.theme.onColor(root.theme.accent) : root.theme.accent)
                             : (tabMouse.containsMouse ? root.theme.fg : root.theme.textMuted)
            inverse: isCurrent && root.activeFocus
            inverseColor: root.theme.accent

            MouseArea {
                id: tabMouse

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.activated(tab.index)
            }
        }
    }

    // Rule with the heavy segment under the current tab.
    Rectangle {
        x: 0
        y: root.tm.lineY(1)
        width: root.tm.cx(root.cells)
        height: root.tm.lw
        color: root.theme.borderInactive
        antialiasing: false
    }
    Rectangle {
        visible: root.current >= 0 && root.current < root.labels.length
        x: root.tm.cx(root.starts[root.current] || 0)
        y: root.tm.lineY(1) - Math.floor(root.tm.lwDev / 2) / root.tm.dpr
        width: root.tm.cx(root.labels.length > 0 ? root.tm.len(root.entry(Math.max(0, root.current))) : 0)
        height: root.tm.lwHeavy
        color: root.theme.accent
        antialiasing: false
    }
}
