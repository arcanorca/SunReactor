/*
 * A Plasma slider drawn with theme tokens: track in the theme's line colour,
 * fill in its accent, square for pixel and terminal renderers, rounded by the
 * theme radius otherwise. Behaviour (keys, accessibility, stepping) is the
 * stock PlasmaComponents3.Slider. Pixel skins replace this with discrete cells
 * inside a PixelSurface (DESIGN.md section 4, pixel rule 3).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.components as PlasmaComponents3

import "../engine"

PlasmaComponents3.Slider {
    id: control

    required property Theme theme
    property color fillColor: theme.accent

    readonly property bool square: !theme.isVector
    readonly property real trackHeight: square ? Math.round(theme.grid.unit * 2) : 4

    background: Item {
        x: control.leftPadding
        y: control.topPadding + (control.availableHeight - height) / 2
        width: control.availableWidth
        height: control.trackHeight

        Rectangle {
            anchors.fill: parent
            radius: control.square ? 0 : height / 2
            color: control.theme.line
            antialiasing: !control.square
        }

        Rectangle {
            width: control.visualPosition * parent.width
            height: parent.height
            radius: control.square ? 0 : height / 2
            color: control.enabled ? control.fillColor : control.theme.textMuted
            antialiasing: !control.square
        }
    }

    handle: Rectangle {
        readonly property real size: control.square ? Math.round(control.theme.grid.unit * 6) : 16

        x: control.leftPadding + control.visualPosition * (control.availableWidth - width)
        y: control.topPadding + (control.availableHeight - height) / 2
        width: control.square ? Math.round(size * 0.6) : size
        height: size
        radius: control.square ? 0 : size / 2
        antialiasing: !control.square
        color: control.pressed || control.visualFocus ? control.fillColor : control.theme.fg
        border.width: control.theme.lineWidth
        border.color: control.visualFocus ? control.theme.borderActive : control.theme.bg
        visible: control.enabled
    }
}
