/*
 * Commodore 64 SunTimes: sunrise, solar noon and sunset as a two-line table,
 * the sun's elevation when the setting shows it, then the READY. prompt that
 * closes the weather page. Module contract: modules/Modules.js.
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

    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    C64Style {
        id: style

        theme: root.theme
    }

    readonly property string elevation: client.hasSolarElevation && settings.showSolarElevation
        ? "  SUN " + Math.round(client.solarElevation) + "°" + (client.solarElevation >= 0 ? " UP" : " DOWN")
        : ""

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(settings.domain, "Title of the sun times panel", "Sun")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: body.height + style.padTop + style.padBottom
        regionName: "c64-suntimes"

        readonly property int cols: style.cols(artWidth)

        Column {
            id: body

            x: style.left(surface.artWidth)
            y: style.padTop
            width: surface.cols * style.cw

            C64Text {
                ink: style.hot
                text: "RISE " + PixelFormat.time(root.client.sunriseEpochS)
                    + "  SET " + PixelFormat.time(root.client.sunsetEpochS)
                maximumArtWidth: body.width
                maximumLines: 2
            }
            C64Text {
                ink: style.ink
                text: "NOON " + PixelFormat.time(root.client.solarNoonEpochS) + root.elevation
                maximumArtWidth: body.width
                maximumLines: 2
            }
            C64Text {
                ink: style.ink
                text: "READY. █"
            }
        }
    }
}
