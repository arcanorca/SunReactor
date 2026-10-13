/*
 * A footer button drawn from theme tokens only: icon + label in the theme's
 * foreground, a border in the line colour (focus: the active border), the
 * theme radius (square for pixel and terminal). Pixel themes set the label in
 * the bitmap font. Without a daemon theme the tokens are Plasma's colours.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami

import "../../engine"

QQC2.AbstractButton {
    id: root

    required property Theme theme
    property bool iconOnly: false
    property string toolTip: ""
    /*! Extra content after the label (swatches, lock, ...). */
    default property alias trailing: extra.data
    property alias leading: lead.data

    property bool highlighted: false
    readonly property color ink: root.highlighted ? root.theme.onColor(root.theme.accent) : root.theme.fg

    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: Kirigami.Units.smallSpacing
    leftPadding: iconOnly ? padding : Kirigami.Units.smallSpacing * 2
    rightPadding: leftPadding
    implicitHeight: Math.max(Kirigami.Units.gridUnit * 1.6, implicitContentHeight + topPadding + bottomPadding)
    implicitWidth: implicitContentWidth + leftPadding + rightPadding

    Accessible.role: Accessible.Button
    Accessible.name: text
    Accessible.description: toolTip

    QQC2.ToolTip.visible: hovered && toolTip.length > 0
    QQC2.ToolTip.text: toolTip
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

    background: Rectangle {
        radius: root.theme.radius
        color: root.highlighted ? root.theme.accent
            : (root.down || root.checked) ? root.theme.alpha(root.theme.accent, 0.25)
            : root.hovered ? root.theme.alpha(root.theme.fg, 0.08) : "transparent"
        border.width: root.theme.isTerminal && !root.visualFocus ? 0 : root.theme.lineWidth
        border.color: root.visualFocus ? root.theme.borderActive : root.theme.line
    }

    contentItem: RowLayout {
        spacing: Kirigami.Units.smallSpacing

        Row {
            id: lead
            visible: children.length > 0
            spacing: Kirigami.Units.smallSpacing
        }
        Kirigami.Icon {
            visible: root.icon.name.length > 0
            Layout.preferredWidth: Kirigami.Units.iconSizes.small
            Layout.preferredHeight: Kirigami.Units.iconSizes.small
            source: root.icon.name
            isMask: root.theme.available
            color: root.ink
        }
        PlasmaComponents3.Label {
            visible: !root.iconOnly && root.text.length > 0 && !bitmapLabel.visible
            text: root.theme.isTerminal ? "[" + root.text + "]" : root.text
            textFormat: Text.PlainText
            color: root.ink
            font.family: root.theme.available ? root.theme.bodyFont : Kirigami.Theme.defaultFont.family
        }
        BitmapText {
            id: bitmapLabel
            visible: !root.iconOnly && root.text.length > 0 && root.theme.isPixel && root.theme.bitmapFont.length > 0
            theme: root.theme
            text: root.text.toUpperCase()
            color: root.ink
        }
        Row {
            id: extra
            visible: children.length > 0
            spacing: Kirigami.Units.smallSpacing
        }
    }
}
