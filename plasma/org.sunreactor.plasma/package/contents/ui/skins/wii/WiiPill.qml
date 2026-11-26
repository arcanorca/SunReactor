/*
 * Wii Menu glossy capsule button backed by 9-slice BorderImage assets
 * (wii_pill_idle.png, wii_pill_outline.png, wii_pill_fill.png).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property WiiStyle style
    property string label: ""
    property string hint: ""
    property bool current: false
    property bool called: false
    property string variant: "fill"
    property bool arrow: false
    property bool focused: false
    readonly property bool hovered: mouse.containsMouse

    signal clicked()
    signal arrowClicked()

    readonly property bool filled: variant === "fill" && current
    readonly property real radius: height / 2
    readonly property color textColor: filled ? style.paper
        : (variant === "fill" ? style.ink : (current ? style.ink : style.inkSoft))

    implicitWidth: row.implicitWidth + 2 * style.gap2 * 1.5
    implicitHeight: style.pillHeight
    opacity: enabled ? 1 : 0.5

    BorderImage {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.filled ? "assets/wii_pill_fill.png"
            : (root.current || root.called ? "assets/wii_pill_outline.png" : "assets/wii_pill_idle.png"))
        border { left: 14; top: 12; right: 14; bottom: 12 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }

    Rectangle {
        visible: root.focused
        property bool auditIgnore: true
        x: -root.style.line2 * 2
        y: -root.style.line2 * 2
        width: root.width + root.style.line2 * 4
        height: root.height + root.style.line2 * 4
        radius: root.radius + root.style.line2 * 2
        antialiasing: true
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.accent
    }

    Rectangle {
        visible: root.hovered && !root.filled
        anchors.fill: parent
        radius: root.radius
        antialiasing: true
        color: root.style.alpha(root.style.accent, 0.08)
    }

    Row {
        id: row

        anchors.centerIn: parent
        spacing: root.style.gap

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: root.label
            textFormat: Text.PlainText
            font: root.style.bodyBoldFont
            color: root.textColor
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: text.length > 0
            text: root.hint
            textFormat: Text.PlainText
            font: root.style.captionFont
            color: root.filled ? root.style.paper : root.style.inkSoft
        }
        Kirigami.Icon {
            id: chevron

            visible: root.arrow
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(root.style.unit * 0.6)
            height: width
            source: "arrow-down"
            isMask: true
            color: root.textColor
        }
    }

    MouseArea {
        id: mouse

        anchors.fill: parent
        hoverEnabled: true
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: root.clicked()
    }
    MouseArea {
        visible: root.arrow
        x: chevron.x - root.style.gap
        y: 0
        width: chevron.width + 2 * root.style.gap
        height: root.height
        cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: root.arrowClicked()
    }
}
