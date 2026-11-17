/*
 * A read-only, selectable command line with a Copy button next to it. The copy
 * goes through the field itself (selectAll, copy, deselect): QML has no
 * clipboard API. The button shows a checkmark for a moment afterwards.
 */

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami

RowLayout {
    id: root

    property string command: ""
    property string domain: ""
    readonly property alias copied: feedback.running
    readonly property alias field: field

    function copy(): void {
        field.forceActiveFocus();
        field.selectAll();
        field.copy();
        field.deselect();
        feedback.restart();
    }

    spacing: Kirigami.Units.smallSpacing

    QQC2.TextField {
        id: field

        Layout.fillWidth: true
        Layout.maximumWidth: implicitWidth
        Layout.minimumWidth: 0
        text: root.command
        readOnly: true
        selectByMouse: true
        horizontalAlignment: TextInput.AlignHCenter
        leftPadding: Kirigami.Units.largeSpacing
        rightPadding: Kirigami.Units.largeSpacing
        topPadding: Kirigami.Units.smallSpacing
        bottomPadding: Kirigami.Units.smallSpacing
        font: Kirigami.Theme.fixedWidthFont
        color: Kirigami.Theme.textColor
        selectionColor: Kirigami.Theme.highlightColor
        selectedTextColor: Kirigami.Theme.highlightedTextColor
        Accessible.name: root.command

        // Themed by hand: the default frame can paint a light box under light text.
        background: Rectangle {
            radius: Kirigami.Units.cornerRadius
            color: Kirigami.Theme.alternateBackgroundColor
            border.width: 1
            border.color: field.activeFocus ? Kirigami.Theme.highlightColor
                : Qt.alpha(Kirigami.Theme.textColor, 0.25)
        }
    }

    PlasmaComponents3.ToolButton {
        id: button

        icon.name: root.copied ? "checkmark" : "edit-copy"
        display: QQC2.AbstractButton.IconOnly
        text: i18nd(root.domain, "Copy command")
        Accessible.name: text
        PlasmaComponents3.ToolTip.visible: hovered || pressed
        PlasmaComponents3.ToolTip.text: root.copied ? i18nd(root.domain, "Copied") : text
        onClicked: root.copy()
    }

    Timer {
        id: feedback

        interval: 1500
    }
}
