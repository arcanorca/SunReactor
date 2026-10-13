/*
 * A native segmented control: a capsule track, the selected segment as a lighter
 * rounded rect with a soft shadow, and 1 px separators that fade next to the
 * selection. Used for the page tabs, the Automatic | Manual | Paused choice and
 * the display picker.
 *
 *   Segmented { style: root.style; segments: [{ id, text }]; current: "automatic"
 *               onPicked: id => ... }
 *
 * `pageTabs` gives the PageTab role and makes Left and Right pick (the page
 * changes as the focus moves); otherwise Left and Right only move the focus and
 * Space or Enter picks. A segment whose id is `arrowId` shows a chevron: a click
 * or Down on it emits arrowPicked(id, segment) for a menu to open from.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

FocusScope {
    id: root

    required property MacStyle style
    /*! [{ id, text }] in order. */
    property var segments: []
    property string current: ""
    property bool pageTabs: false
    property string arrowId: ""

    readonly property int count: segments.length

    signal picked(string id)
    signal arrowPicked(string id, Item anchorItem)

    implicitWidth: row.implicitWidth + 2 * root.style.snap(2)
    implicitHeight: style.segmentHeight

    function focusSegment(index) {
        const cell = cells.itemAt((index + count) % count);
        if (cell) {
            cell.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    function step(index, delta) {
        const to = (index + delta + count) % count;
        if (root.pageTabs) {
            root.picked(root.segments[to].id);
        }
        root.focusSegment(to);
    }

    // The capsule.
    Rectangle {
        anchors.fill: parent
        radius: height / 2
        antialiasing: true
        color: root.style.segmentTrack
    }

    RowLayout {
        id: row

        anchors.fill: parent
        anchors.margins: root.style.snap(2)
        spacing: 0

        Repeater {
            id: cells

            model: root.segments

            delegate: Item {
                id: cell

                required property var modelData
                required property int index

                readonly property string choiceId: modelData.id
                readonly property bool lit: root.current === choiceId
                readonly property bool afterLit: index > 0 && root.current === root.segments[index - 1].id
                readonly property bool hasArrow: root.arrowId === choiceId

                // Its own width is its label's, so the control sizes to its text.
                implicitWidth: legend.implicitWidth + 2 * root.style.padRow

                Layout.fillWidth: true
                Layout.fillHeight: true
                activeFocusOnTab: true

                Accessible.role: root.pageTabs ? Accessible.PageTab : Accessible.RadioButton
                Accessible.name: modelData.text
                Accessible.checkable: true
                Accessible.checked: lit
                Accessible.onPressAction: root.picked(choiceId)

                Keys.onLeftPressed: root.step(index, -1)
                Keys.onRightPressed: root.step(index, 1)
                Keys.onPressed: event => {
                    if (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        root.picked(choiceId);
                        event.accepted = true;
                    } else if (event.key === Qt.Key_Down && hasArrow) {
                        root.arrowPicked(choiceId, cell);
                        event.accepted = true;
                    }
                }

                // The selected segment: a shadow one pixel below, then the face.
                Rectangle {
                    visible: cell.lit
                    x: 0
                    y: root.style.snap(1)
                    width: parent.width
                    height: parent.height
                    radius: root.style.radiusSegment
                    antialiasing: true
                    color: root.style.segmentShadow
                }
                Rectangle {
                    anchors.fill: parent
                    visible: cell.lit
                    radius: root.style.radiusSegment
                    antialiasing: true
                    color: root.style.segmentOn
                }

                // Separator on the left edge, gone next to the selection.
                Rectangle {
                    visible: cell.index > 0
                    opacity: cell.lit || cell.afterLit ? 0 : 1
                    x: 0
                    anchors.verticalCenter: parent.verticalCenter
                    width: root.style.hairline
                    height: parent.height * 0.5
                    color: root.style.separator
                }

                Row {
                    id: legend

                    anchors.centerIn: parent
                    spacing: 4

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: cell.modelData.text
                        textFormat: Text.PlainText
                        font: cell.lit ? root.style.bodyMedium : root.style.body
                        color: cell.lit ? root.style.ink : root.style.inkSecondary
                        Accessible.ignored: true
                    }

                    Chevron {
                        visible: cell.hasArrow
                        anchors.verticalCenter: parent.verticalCenter
                        size: 7
                        color: cell.lit ? root.style.ink : root.style.inkSecondary
                    }
                }

                // Keyboard focus: an accent ring round the segment.
                Rectangle {
                    visible: cell.activeFocus
                    anchors.fill: parent
                    color: "transparent"
                    radius: root.style.radiusSegment
                    border.width: root.style.hairline * 2
                    border.color: root.style.accent
                    antialiasing: true
                }

                MouseArea {
                    id: mouse

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        cell.forceActiveFocus(Qt.MouseFocusReason);
                        root.picked(cell.choiceId);
                    }
                }

                // The chevron's own hit area, above the segment.
                MouseArea {
                    visible: cell.hasArrow
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: root.style.segmentHeight * 1.2
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.arrowPicked(cell.choiceId, cell)
                }
            }
        }
    }
}
