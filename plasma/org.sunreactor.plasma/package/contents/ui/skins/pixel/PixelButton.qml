/*
 * Button, tab or tag inside a PixelSurface (art px), in the style's idiom:
 *
 *   variant "button"  framed (GB box, System 1 rounded rect, Workbench bevel,
 *                     PETSCII reverse video, Nothing pill, neon outline);
 *                     pressed or focused = reverse video
 *   variant "tab"     checkable; checked = reverse video, else plain text
 *   variant "tag"     not interactive: a reverse-video status tag
 *
 * Space / Enter / Return activate; a focus ring shows keyboard focus.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"
import "PixelChrome.js" as Chrome

Item {
    id: root

    required property Theme theme
    required property PixelStyle style
    property string text: ""
    property string variant: "button"
    property bool checked: false
    /*! Tag colour (variant "tag"). */
    property color tagColor: style.ink
    property string accessibleName: text

    signal clicked()

    readonly property bool interactive: variant !== "tag" && enabled
    readonly property bool reversed: variant === "tag" || (variant === "tab" ? checked : (mouse.pressed || activeFocus))
        || (variant === "button" && style.kind === "c64")
    readonly property int padX: variant === "tag" ? 2 : 4
    readonly property color fill: variant === "tag" ? tagColor
        : (style.kind === "amiga" ? theme.borderActive : (style.kind === "neon" || style.kind === "dots" ? style.hot : style.ink))

    implicitWidth: label.artWidth + 2 * padX
    // Two art px of air above and below the capitals, for every variant.
    implicitHeight: label.capHeight + 4
    width: implicitWidth
    height: implicitHeight
    activeFocusOnTab: interactive

    Accessible.role: variant === "tab" ? Accessible.PageTab : (variant === "tag" ? Accessible.StaticText : Accessible.Button)
    Accessible.name: accessibleName
    Accessible.checkable: variant === "tab"
    Accessible.checked: checked
    Accessible.onPressAction: if (root.interactive) root.clicked()

    Keys.onPressed: event => {
        if (interactive && (event.key === Qt.Key_Space || event.key === Qt.Key_Return || event.key === Qt.Key_Enter)) {
            event.accepted = true;
            root.clicked();
        }
    }

    // Frame (buttons only) or the reverse-video block.
    Repeater {
        model: {
            const out = [];
            const W = root.width;
            const H = root.height;
            if (root.reversed) {
                if (root.style.kind === "mac" || root.style.kind === "dots") {
                    Chrome.push(out, 1, 0, W - 2, H, root.fill);
                    Chrome.push(out, 0, 1, W, H - 2, root.fill);
                } else {
                    Chrome.push(out, 0, 0, W, H, root.fill);
                }
                return out;
            }
            if (root.variant !== "button") {
                return out;
            }
            switch (root.style.kind) {
            case "amiga":
                Chrome.push(out, 0, 0, W, 1, root.theme.fg);
                Chrome.push(out, 0, 0, 1, H, root.theme.fg);
                Chrome.push(out, 0, H - 1, W, 1, root.theme.bg);
                Chrome.push(out, W - 1, 0, 1, H, root.theme.bg);
                break;
            case "mac":
            case "dots":
                Chrome.ring(out, 0, 0, W, H, root.style.kind === "mac" ? root.style.ink : root.theme.fg, 2);
                break;
            case "neon":
                Chrome.ring(out, 0, 0, W, H, root.style.hot, 1);
                break;
            default:
                Chrome.ring(out, 0, 0, W, H, root.style.kind === "casio" ? root.theme.borderActive : root.style.ink, 1);
            }
            return out;
        }

        delegate: Rectangle {
            required property var modelData
            x: modelData.x
            y: modelData.y
            width: modelData.w
            height: modelData.h
            color: modelData.c
            antialiasing: false
        }
    }

    // Unchecked tab: a quiet underline.
    Rectangle {
        visible: root.variant === "tab" && !root.checked
        x: 0
        y: root.height - 1
        width: root.width
        height: 1
        color: root.style.ghost
        antialiasing: false
    }

    PixelLabel {
        id: label

        theme: root.theme
        x: root.padX
        y: Math.floor((root.height - capHeight) / 2)
        text: root.text
        face: root.style.bodyFace
        capitals: root.style.kind !== "mac" && root.style.kind !== "amiga"
        color: root.reversed ? (root.style.kind === "amiga" ? root.theme.bg : (root.variant === "tag" ? root.style.paper : root.style.inverse))
            : (root.style.kind === "neon" ? root.style.hot : root.style.ink)
    }

    // Focus ring for tabs/tags-less styles where reverse video is not shown.
    Item {
        visible: root.activeFocus && root.variant === "tab"
        x: -2
        y: -2
        width: root.width + 4
        height: root.height + 4
        Rectangle { x: 0; y: 0; width: parent.width; height: 1; color: root.style.hot; antialiasing: false }
        Rectangle { x: 0; y: parent.height - 1; width: parent.width; height: 1; color: root.style.hot; antialiasing: false }
        Rectangle { x: 0; y: 0; width: 1; height: parent.height; color: root.style.hot; antialiasing: false }
        Rectangle { x: parent.width - 1; y: 0; width: 1; height: parent.height; color: root.style.hot; antialiasing: false }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        enabled: root.interactive
        cursorShape: Qt.PointingHandCursor
        onClicked: {
            root.forceActiveFocus(Qt.MouseFocusReason);
            root.clicked();
        }
    }
}
