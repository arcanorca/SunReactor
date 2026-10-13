/*
 * Footer part "refresh": re-evaluate now.
 */

import QtQuick

import ".."

FooterButton {
    required property PopupFooter footer

    theme: footer.theme
    iconOnly: true
    icon.name: "view-refresh"
    text: i18ndc(footer.domain, "@action:button", "Update Now")
    toolTip: i18nd(footer.domain, "Re-read the sun position and weather, then apply the result")
    enabled: footer.client.isConnected
    onClicked: footer.client.runOnce(true)
}
