/*
 * A neon outline pill: a caps legend in a 1 device-px outline. Filled, it is a
 * lit segment (dark text on the neon). Used for the automation switch, the
 * display tags and the chart's display chooser.
 *
 * `arrow` adds a down chevron on the right with its own hit area (opens a menu
 * in the automation switch); a click elsewhere is `clicked`.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property SynthStyle style
    property string text: ""
    property color tone: style.pink
    property bool filled: false
    property bool arrow: false
    /*! Keyboard focus ring (the caller sets it from its focus). */
    property bool ringed: false

    /*! The chevron's hit area was clicked. */
    signal arrowClicked()
    signal clicked()

    readonly property real padX: root.style.snap(root.style.gap2 * 1.4)
    readonly property real padY: root.style.snap(root.style.gap * 1.2)

    implicitWidth: row.implicitWidth + 2 * padX
    implicitHeight: Math.max(label.implicitHeight, root.style.sizeLabel * 1.6) + 2 * padY
    activeFocusOnTab: true

    Accessible.role: Accessible.Button
    Accessible.name: root.text

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        antialiasing: true
        color: root.filled ? root.tone : root.style.alpha(root.tone, mouse.containsMouse ? 0.12 : 0)
        border.width: root.style.hairline
        border.color: root.tone
    }

    // Keyboard focus: a second, fuller outline.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -root.style.snap(root.style.gap * 0.6)
        visible: root.ringed || root.activeFocus
        radius: height / 2
        color: "transparent"
        border.width: root.style.hairline
        border.color: root.style.alpha(root.tone, 0.7)
        antialiasing: true
    }

    Row {
        id: row

        anchors.centerIn: parent
        spacing: root.style.gap

        Text {
            id: label

            anchors.verticalCenter: parent.verticalCenter
            text: root.text
            textFormat: Text.PlainText
            font: root.style.labelFont
            color: root.filled ? root.style.onNeon : root.tone
        }

        Kirigami.Icon {
            visible: root.arrow
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(root.style.sizeLabel)
            height: width
            source: "arrow-down"
            isMask: true
            color: root.filled ? root.style.onNeon : root.tone
        }
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }

    // The chevron's own target, on top of the pill's: the chevron and its padding only.
    MouseArea {
        visible: root.arrow
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: parent.bottom
        width: Math.min(root.implicitWidth * 0.4, root.style.unit * 2)
        cursorShape: Qt.PointingHandCursor
        onClicked: root.arrowClicked()
    }
}
