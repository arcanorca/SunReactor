/*
 * A Row that a layout can place. Its size hint is ceiled to whole device
 * pixels: a Row of Texts sums to a fractional width, a layout that snaps its
 * allocation to pixels disagrees with that hint, and at a fractional scale the
 * page polishes forever, one pixel back and forth per frame. Children are
 * placed in the inner Row (anchors to `parent` and to siblings work as in a Row).
 */

import QtQuick

Item {
    id: root

    default property alias content: row.data
    property alias spacing: row.spacing
    readonly property real dpr: Math.max(1, Screen.devicePixelRatio)

    implicitWidth: Math.ceil(row.implicitWidth * dpr - 0.001) / dpr
    implicitHeight: Math.ceil(row.implicitHeight * dpr - 0.001) / dpr

    Row {
        id: row
    }
}
