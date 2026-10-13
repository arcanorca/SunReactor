/*
 * A compact status tag - tiny caps in a hairline box, like the mode legends
 * of a bench instrument: [LIVE] [MAN] [OFF]. `filled` inverts it (the active
 * state); `interactive` makes it a checkable push tag (channel selectors).
 * Sizes are whole device pixels so the box edges stay one crisp pixel.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T

T.AbstractButton {
    id: root

    required property InstrumentStyle style
    property color tone: style.inkMuted
    property bool filled: false
    property bool interactive: false
    /*! Draw only the text (no box) while not filled: quieter selectors. */
    property bool bare: false

    readonly property bool lit: filled
    readonly property real padX: style.snap(style.sizeLegend * 0.55)
    readonly property real padY: style.snap(Math.max(1, style.sizeLegend * 0.18))

    implicitWidth: style.snap(label.implicitWidth + padX * 2)
    implicitHeight: style.snap(label.implicitHeight + padY * 2)
    focusPolicy: interactive ? Qt.TabFocus : Qt.NoFocus
    hoverEnabled: interactive
    enabled: interactive
    checkable: false
    Accessible.role: interactive ? Accessible.Button : Accessible.StaticText
    Accessible.checkable: interactive
    Accessible.checked: interactive && filled
    Accessible.name: text

    background: Rectangle {
        color: root.lit ? root.tone : "transparent"
        radius: root.style.tagRadius
        antialiasing: radius > 0
        border.width: root.bare && !root.lit && !root.visualFocus ? 0 : root.style.hairline
        border.color: root.visualFocus ? root.style.theme.borderActive
            : (root.hovered && !root.lit ? root.style.ink : root.tone)
    }

    contentItem: Text {
        id: label

        text: root.text
        textFormat: Text.PlainText
        font: root.style.legendFont
        color: root.lit ? root.style.theme.onColor(root.tone) : (root.hovered ? root.style.ink : root.tone)
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        leftPadding: root.padX
        rightPadding: root.padX
        topPadding: root.padY
        bottomPadding: root.padY
    }
}
