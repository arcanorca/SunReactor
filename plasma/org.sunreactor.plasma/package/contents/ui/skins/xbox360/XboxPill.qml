/*
 * Glossy Xbox 360 pill button backed by 9-slice BorderImage assets
 * (xbox_pill_active.png / xbox_pill_idle.png).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property XboxStyle style
    property string label: ""
    property string hint: ""
    property bool current: false
    property bool called: false
    property bool arrow: false
    property bool focused: false
    readonly property bool hovered: mouse.containsMouse

    signal clicked()
    signal arrowClicked()

    implicitWidth: row.implicitWidth + 2 * style.gap2 * 0.9
    implicitHeight: style.pillHeight
    opacity: enabled ? 1 : 0.5

    BorderImage {
        anchors.fill: parent
        source: Qt.resolvedUrl(root.style.asset(root.current ? "xbox_pill_active.png"
            : ((root.called || root.hovered) ? "xbox_pill_called.png" : "xbox_pill_idle.png")))
        border { left: 14; top: 10; right: 14; bottom: 10 }
        horizontalTileMode: BorderImage.Stretch
        verticalTileMode: BorderImage.Stretch
        smooth: true
    }

    Rectangle {
        visible: root.focused
        property bool auditIgnore: true
        x: -root.style.line2
        y: -root.style.line2
        width: root.width + 2 * root.style.line2
        height: root.height + 2 * root.style.line2
        radius: Math.min(height / 2, 14)
        color: "transparent"
        border.width: root.style.line2
        border.color: root.style.greenBright
        antialiasing: true
    }

    Row {
        id: row

        anchors.centerIn: parent
        spacing: root.style.gap

        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: text.length > 0
            text: root.label
            textFormat: Text.PlainText
            font: root.style.bodyBoldFont
            color: root.current ? "#ffffff" : root.style.ink
        }
        Text {
            anchors.verticalCenter: parent.verticalCenter
            visible: text.length > 0
            text: root.hint
            textFormat: Text.PlainText
            font: root.style.captionBoldFont
            color: root.current ? "#eef8e8" : root.style.muted
        }
        Kirigami.Icon {
            id: chevron

            visible: root.arrow
            anchors.verticalCenter: parent.verticalCenter
            width: root.style.snap(root.style.unit * 0.6)
            height: width
            source: "arrow-down"
            isMask: true
            color: root.current ? "#ffffff" : root.style.ink
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
