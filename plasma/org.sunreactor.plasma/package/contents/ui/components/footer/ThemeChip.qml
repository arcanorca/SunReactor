/*
 * Footer part "theme": which theme is showing (swatches + name), a lock when
 * this widget pinned it, a warning when the daemon had to fall back
 * (status.theme_warning). Click opens the picker: follow or pin.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import ".."

FooterButton {
    id: root

    required property PopupFooter footer

    readonly property string domain: footer.domain
    readonly property bool warned: footer.client.themeWarning.length > 0

    theme: footer.theme
    visible: footer.theme.available || warned
    text: footer.theme.name
    checked: picker.opened
    toolTip: picker.opened ? "" : (warned ? footer.client.themeWarning
        : (footer.pinned ? i18nd(domain, "Pinned for this widget. Click to choose another theme.")
                         : i18nd(domain, "Same as the terminal app. Click to pin another theme.")))
    Accessible.role: Accessible.ButtonMenu
    Accessible.name: footer.pinned
        ? i18ndc(domain, "Accessible name; placeholder is a theme name", "Theme: %1, pinned", footer.theme.name)
        : i18ndc(domain, "Accessible name; placeholder is a theme name", "Theme: %1", footer.theme.name)

    leading: ThemeSwatches {
        anchors.verticalCenter: parent ? parent.verticalCenter : undefined
        colors: [root.theme.bg, root.theme.fg, root.theme.accent, root.theme.secondaryAccent]
        outline: root.theme.line
    }

    Kirigami.Icon {
        visible: root.footer.pinned
        width: Kirigami.Units.iconSizes.small
        height: width
        source: "object-locked"
        isMask: true
        color: root.ink
    }
    Kirigami.Icon {
        visible: root.warned
        width: Kirigami.Units.iconSizes.small
        height: width
        source: "dialog-warning"
        isMask: true
        color: root.theme.warning
    }

    onClicked: picker.opened ? picker.close() : picker.open()

    ThemePicker {
        id: picker
        footer: root.footer
        x: root.width - width
        y: -height - Kirigami.Units.smallSpacing
    }
}
