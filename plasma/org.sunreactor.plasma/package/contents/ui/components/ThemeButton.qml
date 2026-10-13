/*
 * The theme button: a square at the right end of the tab row, the first thing
 * under the status line. Its glyph is the active theme's own palette, a 2x2
 * swatch (background, foreground, accent, secondary accent); a one-device-pixel
 * frame in the text-muted colour appears when the theme draws borders. A corner
 * badge tells how the widget takes its theme: a dot while it follows the terminal
 * app, a lock when pinned, a warning sign when the daemon fell back. Click, Space
 * or Return opens ThemePicker under it. Tokens only, so every renderer draws the
 * same glyph; the frame, the hover fill and the corner radius follow the theme.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../skins/nothing"

QQC2.AbstractButton {
    id: root

    required property Theme theme
    required property SunReactorClient client
    /*! Plasmoid config: "follow" or "pinned", and the pinned theme id. */
    property string themeMode: "follow"
    property string pinnedId: ""
    /*! The daemon's list_themes entries; a host may supply its own list (the tests do). */
    property var themeList: client.themeList

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    /*! The Nothing flagship draws a 24 px round hairline with a dot matrix (skins/nothing/ThemeGlyph.qml). */
    readonly property bool dotted: theme.themeId === "nothing"
    readonly property bool pinned: themeMode === "pinned" && pinnedId.length > 0
    readonly property bool warned: client.themeWarning.length > 0
    readonly property bool pickerOpen: picker.opened
    /*! The theme the daemon is on: what "follow" means right now. */
    readonly property string followedName: client.theme && client.theme.name ? client.theme.name : client.themeName
    /*! The theme on screen, pinned or followed. */
    readonly property string shownName: theme.available ? theme.name : i18nd(domain, "Plasma colours")
    readonly property string stateText: pinned
        ? i18ndc(domain, "Tooltip; placeholder is a theme name", "Theme: %1 · pinned", shownName)
        : i18ndc(domain, "Tooltip; placeholder is a theme name", "Theme: %1 · follows the terminal", shownName)

    signal followRequested()
    signal pinRequested(string id)

    /*! Opens the picker (also the screenshot harness's entry point). */
    function openPicker() {
        picker.open();
    }
    /*! Re-anchors an open picker after the layout above it changed. */
    function relayout() {
        picker.place();
    }

    /*! A length in whole device pixels; in the pixel renderer, whole art pixels. */
    function snapped(logical) {
        return theme.isPixel ? Math.max(1, Math.round(logical / theme.grid.unit)) * theme.grid.unit
                             : theme.grid.snap(logical);
    }

    readonly property real box: snapped(24)
    readonly property real inset: snapped((implicitWidth - box) / 2)
    readonly property real cell: snapped(6.5)
    readonly property real badgeSize: snapped(10)

    focusPolicy: Qt.StrongFocus
    hoverEnabled: true
    padding: 0
    implicitWidth: dotted ? box : snapped(28)
    implicitHeight: dotted ? box : snapped(28)

    Accessible.role: Accessible.Button
    Accessible.name: i18ndc(domain, "Accessible name; placeholder is a theme name", "Theme: %1", shownName)
    Accessible.description: pinned
        ? i18nd(domain, "Pinned for this widget. Activate to choose another theme.")
        : i18nd(domain, "Follows the terminal app. Activate to choose a theme.")

    QQC2.ToolTip.visible: hovered && !picker.opened
    QQC2.ToolTip.text: root.warned ? root.stateText + "\n" + root.client.themeWarning : root.stateText
    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

    // The one toggle: a press outside the picker never closes it on the button.
    onClicked: picker.toggle()

    background: Rectangle {
        x: root.inset
        y: root.inset
        width: root.box
        height: root.box
        radius: root.dotted ? width / 2 : root.theme.radius
        antialiasing: root.dotted
        color: root.dotted
            ? (root.pickerOpen || root.down ? root.theme.alpha(root.theme.fg, 0.12)
               : root.hovered ? root.theme.alpha(root.theme.fg, 0.06) : "transparent")
            : (root.pickerOpen || root.down ? root.theme.alpha(root.theme.accent, 0.25)
               : root.hovered ? root.theme.alpha(root.theme.fg, 0.08) : "transparent")
        border.width: root.dotted ? root.theme.hairline
            : (root.theme.border === "none" && !root.visualFocus ? 0 : root.theme.lineWidth)
        border.color: root.dotted
            ? (root.visualFocus || root.hovered || root.pickerOpen ? root.theme.fg : root.theme.borderInactive)
            : (root.visualFocus || root.hovered || root.pickerOpen ? root.theme.accent : root.theme.line)
    }

    /*! The Nothing glyph: the palette as a dot matrix and the dot badge. */
    Component {
        id: dottedGlyph

        ThemeGlyph {
            theme: root.theme
            pinned: root.pinned
            warned: root.warned
        }
    }

    contentItem: Item {
        // Keyboard focus: an accent ring round the whole square (ink for Nothing).
        Rectangle {
            visible: root.visualFocus
            anchors.fill: parent
            color: "transparent"
            border.width: root.theme.isPixel ? root.theme.grid.unit : 2 * root.theme.lineWidth
            border.color: root.dotted ? root.theme.fg : root.theme.accent
            radius: root.theme.radius > 0 ? root.theme.radius + root.inset : 0
            antialiasing: radius > 0
        }

        Loader {
            anchors.fill: parent
            active: root.dotted
            sourceComponent: dottedGlyph
        }

        // The swatch, centred on whole device pixels.
        Item {
            id: swatch

            visible: !root.dotted
            x: root.snapped((parent.width - width) / 2)
            y: root.snapped((parent.height - height) / 2)
            width: root.cell * 2
            height: width

            // The frame: one device pixel just outside the swatch, when the theme has borders.
            Rectangle {
                visible: root.theme.border !== "none"
                x: -root.theme.hairline
                y: -root.theme.hairline
                width: swatch.width + 2 * root.theme.hairline
                height: width
                color: "transparent"
                border.width: root.theme.hairline
                border.color: root.theme.textMuted
                radius: root.theme.radius
                antialiasing: radius > 0
            }

            Rectangle {
                x: 0
                y: 0
                width: root.cell
                height: width
                color: root.theme.bg
                antialiasing: false
            }
            Rectangle {
                x: root.cell
                y: 0
                width: root.cell
                height: width
                color: root.theme.fg
                antialiasing: false
            }
            Rectangle {
                x: 0
                y: root.cell
                width: root.cell
                height: width
                color: root.theme.accent
                antialiasing: false
            }
            Rectangle {
                x: root.cell
                y: root.cell
                width: root.cell
                height: width
                color: root.theme.secondaryAccent
                antialiasing: false
            }
        }

        // The corner badge, on a ring in the surface colour so it reads over the frame.
        Rectangle {
            visible: !root.dotted
            x: parent.width - width
            y: 0
            width: root.badgeSize
            height: width
            radius: root.theme.isVector ? width / 2 : 0
            antialiasing: radius > 0
            color: root.theme.bg
            border.width: root.theme.hairline
            border.color: root.theme.line

            // Following the terminal app: a dot in the success colour.
            Rectangle {
                visible: !root.warned && !root.pinned
                anchors.centerIn: parent
                width: root.snapped(4)
                height: width
                radius: root.theme.isVector ? width / 2 : 0
                color: root.theme.success
            }
            // Pinned for this widget: a lock.
            Kirigami.Icon {
                visible: root.pinned && !root.warned
                anchors.centerIn: parent
                width: root.snapped(8)
                height: width
                source: "object-locked"
                isMask: true
                color: root.theme.fg
            }
            // The daemon fell back to another theme: a warning sign.
            Kirigami.Icon {
                visible: root.warned
                anchors.centerIn: parent
                width: root.snapped(8)
                height: width
                source: "dialog-warning"
                isMask: true
                color: root.theme.warning
            }
        }
    }

    ThemePicker {
        id: picker

        anchorItem: root
        theme: root.theme
        client: root.client
        domain: root.domain
        followedName: root.followedName
        pinned: root.pinned
        pinnedId: root.pinnedId
        themes: root.themeList

        onFollowRequested: root.followRequested()
        onPinRequested: id => root.pinRequested(id)
    }
}
