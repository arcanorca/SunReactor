/*
 * Amiga Telemetry: the Workbench screen title bar, white on black, two lines:
 * "SunReactor Workbench" over the chip-memory line, with the daemon state and the
 * clock at its right. Hidden by default (the Telemetry module is off). Module
 * contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel/PixelFormat.js" as PixelFormat

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected
    readonly property var clock: PixelFormat.timeParts(client.nowEpochS)
    readonly property string stateText: {
        switch (root.client.mode) {
        case SunReactorClient.Paused:
            return i18ndc(root.domain, "Daemon state in the title bar", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(root.domain, "Daemon state in the title bar", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(root.domain, "Daemon state in the title bar", "IDLE");
        default:
            return i18ndc(root.domain, "Daemon state in the title bar", "LIVE");
        }
    }
    readonly property string tempText: client.hasTemperature
        ? i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1°C", Math.round(root.client.temperatureC)) : ""

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: 20
        regionName: "telemetry"

        Rectangle {
            width: surface.artWidth
            height: surface.artHeight
            color: tokens.ink
            antialiasing: false
        }

        AmigaText {
            id: title

            x: 3
            y: 2
            text: i18nd(root.domain, "SunReactor Workbench")
            color: tokens.barText
            maximumArtWidth: surface.artWidth - 6
        }

        AmigaText {
            id: chip

            x: 3
            y: 11
            text: i18nd(root.domain, "Chip mem") + (root.tempText.length > 0 ? " " + root.tempText : "")
            color: tokens.barText
            maximumArtWidth: Math.floor((surface.artWidth - 6) / 2)
        }

        AmigaText {
            id: status

            x: surface.artWidth - 3 - artWidth
            y: 11
            text: root.stateText + " " + root.clock.hm + root.clock.meridiem
            color: tokens.hot
            maximumArtWidth: surface.artWidth - 6 - chip.artWidth - 4
        }
    }
}
