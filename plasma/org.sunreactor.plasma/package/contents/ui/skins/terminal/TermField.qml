/*
 * One focusable label/value row, the TUI's kit::field_line:
 *
 *     ❯ Minimum        ‹  5% ›      cursor row while the list has focus
 *       Maximum           50%
 *
 * The value sits in its own column (`labelCells` > 0) or right-aligned at the
 * end of the row (`labelCells` -1). On the cursor row of a focused list the
 * value becomes a capsule in video inverse with ‹ › (←/→ adjust it); clicking
 * ‹ or › steps it, clicking elsewhere on the row selects it.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "Term.js" as Term

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    property int cells: tm.cols(width)
    property string label: ""
    property string value: ""
    /*! Cells of the label column; -1 puts the value at the end of the row. */
    property int labelCells: -1
    /*! Value width in cells (padded on the left). */
    property int valueCells: 4
    property bool current: false
    property bool active: false
    property bool adjustable: true
    property bool strongLabel: current
    property color labelColor: theme.fg
    property color valueColor: theme.fg

    signal clicked()
    signal stepped(int direction)

    readonly property bool capsule: current && active && adjustable
    /*! First cell of "‹ value ›" (valueCells + 4 wide). */
    readonly property int valueAt: labelCells >= 0 ? 2 + labelCells + 1 : Math.max(2, cells - valueCells - 4)
    readonly property int labelRoom: labelCells >= 0 ? labelCells : Math.max(0, valueAt - 2 - 1)

    width: tm.cx(cells)
    height: tm.ch

    MouseArea {
        anchors.fill: parent
        onClicked: root.clicked()
    }

    TermText {
        tm: root.tm
        visible: root.current
        value: root.active ? "❯" : "›"
        strong: root.active
        color: root.active ? root.theme.accent : root.theme.textMuted
    }

    TermText {
        tm: root.tm
        x: root.tm.cx(2)
        value: root.label
        cells: root.labelRoom
        strong: root.strongLabel
        color: root.labelColor
    }

    TermText {
        tm: root.tm
        visible: root.capsule
        x: root.tm.cx(root.valueAt)
        value: "‹"
        strong: true
        color: root.theme.accent

        MouseArea {
            anchors.fill: parent
            anchors.margins: -2
            onClicked: root.stepped(-1)
        }
    }

    TermText {
        tm: root.tm
        x: root.tm.cx(root.valueAt + 1)
        value: " " + Term.padStart(root.value, root.valueCells) + " "
        strong: root.current
        color: root.capsule ? root.theme.onColor(root.theme.accent) : root.valueColor
        inverse: root.capsule
        inverseColor: root.theme.accent
    }

    TermText {
        tm: root.tm
        visible: root.capsule
        x: root.tm.cx(root.valueAt + root.valueCells + 3)
        value: "›"
        strong: true
        color: root.theme.accent

        MouseArea {
            anchors.fill: parent
            anchors.margins: -2
            onClicked: root.stepped(1)
        }
    }
}
