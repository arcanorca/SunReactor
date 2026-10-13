/*
 * Commodore 64 telemetry: the BASIC boot banner (hidden by default; the
 * widget's module list shows it). The live values are the daemon state and the
 * display count. Module contract: modules/Modules.js.
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

    readonly property bool available: client.isConnected

    C64Style {
        id: style

        theme: root.theme
    }

    readonly property string stateWord: {
        const domain = root.settings.domain;
        if (!root.client.isConnected) {
            return i18ndc(root.domain, "Daemon state in the C64 banner", "OFFLINE");
        }
        switch (root.client.mode) {
        case SunReactorClient.Paused:
            return i18ndc(root.domain, "Daemon state in the C64 banner", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(root.domain, "Daemon state in the C64 banner", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(root.domain, "Daemon state in the C64 banner", "IDLE");
        default:
            return i18ndc(root.domain, "Daemon state in the C64 banner", "LIVE");
        }
    }

    readonly property int displayCount: root.client.monitors ? root.client.monitors.length : 0

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: stateWord

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-telemetry"

        readonly property int cols: style.cols(artWidth)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.hot
                text: "**** SUNREACTOR LUMINANCE V1 ****"
                maximumArtWidth: body.width
                maximumLines: 2
            }
            C64Text {
                ink: style.ink
                text: "64K RAM SYSTEM  " + root.displayCount + " DISPLAYS"
                maximumArtWidth: body.width
                maximumLines: 2
            }
            C64Text {
                ink: style.ink
                text: root.stateWord + "  " + PixelFormat.time(root.client.nowEpochS)
                maximumArtWidth: body.width
                maximumLines: 2
            }
            C64Text {
                ink: style.ink
                text: "READY. █"
                maximumArtWidth: body.width
            }
        }
    }
}
