/*
 * Tab bar, terminal family: the TUI's workspace tabs (chrome.rs render_tabs),
 * numbered labels with the selected page in accent, inverse video while the bar
 * has focus, and a heavy accent segment under it. The weather tab carries its
 * live temperature. Click, 1 / 2, or Left / Right while focused. Mirrors
 * skins/terminal/TermTabs.qml, and adds the accessible page tabs it lacks.
 *
 *     1 Brightness   2 Weather 15°C
 *     ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "../../modules/Modules.js" as Modules
import "../../skins/terminal"

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)

    readonly property int gap: 3

    TermMetrics {
        id: metrics

        theme: root.theme
    }

    readonly property int cellsWide: metrics.cols(width)
    readonly property int currentIndex: tabs.findIndex(tab => tab.id === current)
    readonly property var starts: {
        const out = [];
        let at = 1;
        for (let i = 0; i < tabs.length; ++i) {
            out.push(at);
            at += metrics.len(entry(i)) + gap;
        }
        return out;
    }

    function entry(i) {
        const tab = tabs[i];
        return (i + 1) + " " + tab.text + (tab.hint ? " " + tab.hint : "");
    }

    implicitHeight: metrics.cy(2)
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))
    Keys.onPressed: event => {
        const n = parseInt(event.text, 10);
        if (n >= 1 && n <= root.tabs.length) {
            root.picked(root.tabs[n - 1].id);
            event.accepted = true;
        }
    }

    Repeater {
        model: root.tabs.length

        delegate: TermText {
            id: tab

            required property int index

            readonly property bool isCurrent: index === root.currentIndex
            readonly property var page: root.tabs[index]

            tm: metrics
            x: metrics.cx(root.starts[index])
            value: root.entry(index)
            cells: Math.max(0, root.cellsWide - root.starts[index])
            strong: isCurrent
            color: isCurrent ? root.theme.onColor(root.theme.accent)
                             : (mouse.containsMouse ? root.theme.fg : root.theme.textMuted)
            // The selected page is a capsule, so it is never mistaken for the
            // display tabs of the light cycle below it, which are not inverse.
            inverse: isCurrent
            inverseColor: root.theme.accent

            Accessible.role: Accessible.PageTab
            Accessible.name: page.text
            Accessible.description: page.hint || ""
            Accessible.checkable: true
            Accessible.checked: isCurrent
            Accessible.onPressAction: root.picked(tab.page.id)

            MouseArea {
                id: mouse

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: Qt.PointingHandCursor
                onClicked: root.picked(tab.page.id)
            }
        }
    }

    // The focus caret, as TermTabs draws it while the bar is focused.
    TermText {
        tm: metrics
        visible: root.activeFocus
        value: "❯"
        strong: true
        color: root.theme.accent
    }

    // The rule, with the heavy segment under the selected page.
    Rectangle {
        x: 0
        y: metrics.lineY(1)
        width: metrics.cx(root.cellsWide)
        height: metrics.lw
        color: root.theme.borderInactive
        antialiasing: false
    }
    Rectangle {
        visible: root.currentIndex >= 0
        x: metrics.cx(root.currentIndex >= 0 ? root.starts[root.currentIndex] : 0)
        y: metrics.lineY(1) - Math.floor(metrics.lwDev / 2) / metrics.dpr
        width: metrics.cx(root.currentIndex >= 0 ? metrics.len(root.entry(root.currentIndex)) : 0)
        height: metrics.lwHeavy
        color: root.theme.accent
        antialiasing: false
    }
}
