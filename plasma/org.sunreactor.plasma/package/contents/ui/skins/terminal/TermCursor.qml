/*
 * The keyboard cursor of a list row, the TUI's selection: `❯` in the gutter
 * and, while the list has focus, the row in video inverse (accent behind,
 * background colour on top) like the theme picker's selected line. Without
 * focus only the retained marker `›` stays, muted. Put it under the row's
 * text and colour the text with `textColor`.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

Item {
    id: root

    required property TermMetrics tm
    required property Theme theme

    /*! Row width in cells (the inverse bar spans it). */
    property int cells: 10
    property bool active: true
    /*! The row is the current one of its list. */
    property bool current: true
    property color barColor: theme.accent

    /*! Colour for text drawn on this row. */
    readonly property color textColor: current && active ? theme.onColor(barColor) : theme.fg

    width: tm.cx(cells)
    height: tm.ch

    Rectangle {
        anchors.fill: parent
        visible: root.current && root.active
        color: root.barColor
        antialiasing: false
    }

    TermText {
        tm: root.tm
        visible: root.current
        value: root.active ? "❯" : "›"
        strong: root.active
        color: root.active ? root.textColor : root.theme.textMuted
    }
}
