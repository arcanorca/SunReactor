/*
 * A front-panel push button: tiny caps in a hairline box, a filled face while
 * pressed, the theme's active border on keyboard focus. Used for the few
 * actions modules own (back to automatic, limits).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Templates as T

T.Button {
    id: control

    required property InstrumentStyle style
    property color tone: style.ink

    readonly property real padX: style.snap(style.sizeLegend * 0.8)
    readonly property real padY: style.snap(style.sizeLegend * 0.35)

    implicitWidth: style.snap(label.implicitWidth + padX * 2)
    implicitHeight: style.snap(label.implicitHeight + padY * 2)
    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    Accessible.name: text

    background: Rectangle {
        radius: control.style.tagRadius
        antialiasing: radius > 0
        color: control.down ? control.tone
            : (control.hovered ? control.style.theme.alpha(control.tone, 0.08) : "transparent")
        border.width: control.visualFocus ? control.style.line2 : control.style.hairline
        border.color: control.visualFocus ? control.style.theme.borderActive
            : (control.hovered ? control.tone : control.style.ruleStrong)
    }

    contentItem: Text {
        id: label

        text: control.text
        textFormat: Text.PlainText
        font: control.style.legendFont
        color: control.down ? control.style.theme.onColor(control.tone) : control.tone
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
    }

    leftPadding: padX
    rightPadding: padX
    topPadding: padY
    bottomPadding: padY
}
