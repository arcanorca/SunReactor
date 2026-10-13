/*
 * A round pill: a hairline outline, or a white fill with black text when it is
 * the active choice. `call` adds the red lamp of the one call to action;
 * `arrow` adds a down arrow with its own hit area (`arrowClicked`), the way
 * PAUSED offers its durations. `interactive: false` makes a badge: shown, never
 * pressed. Space and Enter press it; the focus ring sits one gap outside.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: root

    required property NothingStyle style
    property string text: ""
    property bool active: false
    property bool call: false
    property bool arrow: false
    property bool compact: false
    property bool interactive: true
    /*! A choice of a group: exposed to assistive tech as a radio button. */
    property bool radio: false
    property string accessibleName: text
    property string accessibleDescription: ""

    signal clicked()
    signal arrowClicked()

    readonly property color fill: active ? style.ink : "transparent"
    readonly property color ink: active ? style.theme.bg : (interactive ? style.ink : style.muted)

    implicitHeight: Math.round(style.unit * (compact ? 1.3 : 1.9))
    implicitWidth: row.implicitWidth + height * 1.1
    activeFocusOnTab: interactive

    Accessible.role: radio ? Accessible.RadioButton : Accessible.Button
    Accessible.name: accessibleName
    Accessible.description: accessibleDescription
    Accessible.checkable: radio
    Accessible.checked: active
    Accessible.focusable: interactive
    Accessible.onPressAction: {
        if (root.interactive) {
            root.clicked();
        }
    }

    Keys.onPressed: event => {
        if (root.interactive && (event.key === Qt.Key_Space || event.key === Qt.Key_Return
                                 || event.key === Qt.Key_Enter)) {
            root.clicked();
            event.accepted = true;
        }
    }

    // The outline, filled when active; hover lifts the outline to the ink.
    Rectangle {
        anchors.fill: parent
        radius: height / 2
        antialiasing: true
        color: root.fill
        border.width: root.style.hairline
        border.color: root.active || (body.containsMouse && root.interactive) ? root.style.ink : root.style.outline
    }

    MouseArea {
        id: body

        anchors.fill: parent
        enabled: root.interactive
        hoverEnabled: true
        cursorShape: root.interactive ? Qt.PointingHandCursor : Qt.ArrowCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.clicked();
        }
    }

    RowLayout {
        id: row

        anchors.centerIn: parent
        spacing: root.style.gap

        Lamp {
            Layout.alignment: Qt.AlignVCenter
            visible: root.call
            tone: root.style.accent
        }

        Text {
            Layout.alignment: Qt.AlignVCenter
            text: root.text
            textFormat: Text.PlainText
            font: root.style.label
            color: root.ink
        }

        // The arrow has its own hit area, stacked over the body: a click there opens the durations.
        Item {
            id: arrowHit

            Layout.alignment: Qt.AlignVCenter
            visible: root.arrow
            implicitWidth: arrowIcon.width + root.style.gap * 2
            implicitHeight: root.height

            Kirigami.Icon {
                id: arrowIcon

                anchors.centerIn: parent
                width: root.style.snap(root.style.unit * 0.6)
                height: width
                source: "arrow-down"
                isMask: true
                color: root.ink
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.arrowClicked()
            }
        }
    }

    // Keyboard focus: a hairline ring one gap outside the pill.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -root.style.gap * 0.5
        visible: root.activeFocus
        radius: height / 2
        antialiasing: true
        color: "transparent"
        border.width: root.style.hairline
        border.color: root.style.ink
    }
}
