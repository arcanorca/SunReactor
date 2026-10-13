/*
 * Nintendo DS top screen, weather page: the current sky as the pixel sprite on
 * a white card, the temperature in the bitmap face, and the condition. The
 * hinge closes the screen. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../pixel"
import "../pixel/PixelFormat.js" as PixelFormat
import "../pixel/Sprites.js" as Sprites
import "Nds.js" as Nds

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled
    readonly property var clock: PixelFormat.timeParts(client.nowEpochS)

    readonly property string condition: {
        const text = weatherText.conditionLabel(client.weatherCondition);
        return client.hasWeatherReading && text.length > 0 ? text : i18nd(domain, "No weather reading");
    }

    /*! What the daemon's bounded weather multiplier does to the targets, in words. */
    readonly property string effect: {
        if (!client.hasWeatherReading) {
            return client.weatherState === "no_api_key"
                ? i18nd(domain, "No API key is configured")
                : i18nd(domain, "No reading yet");
        }
        if (client.weatherStale) {
            return i18nd(domain, "Last reading is out of date");
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(domain, "Not changing brightness right now");
    }

    WeatherText {
        id: weatherText
    }

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: weatherText.summary(client)

    NdsLid {
        id: surface

        width: parent.width
        theme: root.theme
        grid: root.theme.grid
        fillsWidth: true
        hinge: true
        lidHeight: Nds.WEATHER_TOP_H - Nds.HINGE
        regionName: "weather"

        // Header strip, as on the brightness screen.
        Rectangle {
            x: 2
            y: 2
            width: surface.artWidth - 4
            height: 10
            color: root.theme.bg
            antialiasing: false
        }
        Rectangle {
            x: 2
            y: 11
            width: surface.artWidth - 4
            height: 1
            color: root.theme.borderInactive
            antialiasing: false
        }
        PixelLabel {
            theme: root.theme
            x: 5
            y: 3
            face: "bitmap"
            capitals: true
            color: pstyle.ink
            text: i18ndc(root.domain, "Title of the weather panel", "Weather")
        }
        PixelLabel {
            id: clockLabel

            theme: root.theme
            x: surface.artWidth - 5 - artWidth
            y: 4
            face: "small"
            capitals: true
            color: pstyle.ink
            text: root.clock.hm + root.clock.meridiem
        }

        // The sky on a white card.
        Repeater {
            model: Nds.box([], 6, 15, 20, 20, root.theme.bg, root.theme.borderInactive)
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
        PixelSprite {
            x: 8
            y: 17
            source: Sprites.weatherAtlas()
            frame: root.client.hasWeatherReading
                ? Sprites.conditionFrame(root.client.weatherCondition, root.client.weatherIsNight)
                : Sprites.weatherFrame("unknown")
            colors: pstyle.spriteColors
        }

        BitmapText {
            id: temp

            theme: root.theme
            inSurface: true
            magnification: 2
            x: 32
            y: 13
            color: pstyle.ink
            text: root.client.hasTemperature ? String(Math.round(root.client.temperatureC)) : "--"
        }
        PixelLabel {
            theme: root.theme
            x: temp.x + temp.artWidth + 3
            y: 16
            face: "small"
            color: pstyle.dim
            text: "°C"
        }
        // The condition and the effect, each on a white chip so they read on the menu.
        Repeater {
            model: Nds.box(Nds.box([], 29, 28, surface.artWidth - 33, 9, root.theme.bg, root.theme.borderInactive),
                           4, 38, surface.artWidth - 8, 9, root.theme.bg, root.theme.borderInactive)
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
        PixelLabel {
            theme: root.theme
            x: 32
            y: 30
            face: "small"
            color: pstyle.ink
            text: root.condition
            maximumArtWidth: Math.max(8, surface.artWidth - 36)
        }
        PixelLabel {
            theme: root.theme
            x: 7
            y: 40
            face: "small"
            color: pstyle.ink
            text: root.effect
            maximumArtWidth: Math.max(8, surface.artWidth - 14)
            maximumLines: 1
        }
    }
}
