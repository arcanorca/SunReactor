/*
 * Amiga Weather: a Workbench window "Weather": the sky sprite (the pixel atlas,
 * on the Workbench pens), the temperature as the hero figure, the condition
 * beside it, and the effect on the displays on one line. Module contract:
 * modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../pixel/Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    readonly property string condition: {
        const text = weatherText.conditionLabel(root.client.weatherCondition);
        return root.client.hasWeatherReading && text.length > 0 ? text : i18nd(root.domain, "No weather reading");
    }
    readonly property string effect: {
        if (!root.client.hasWeatherReading) {
            switch (root.client.weatherState) {
            case "no_api_key":
                return i18nd(root.domain, "No API key is configured");
            case "loading":
                return i18nd(root.domain, "Looking up the current conditions…");
            case "unauthorized":
                return i18nd(root.domain, "The weather provider rejected the API key");
            case "rate_limited":
                return i18nd(root.domain, "The weather provider is rate limiting requests");
            case "network_error":
                return i18nd(root.domain, "The weather provider could not be reached");
            default:
                return i18nd(root.domain, "No reading yet");
            }
        }
        if (root.client.weatherStale) {
            return i18nd(root.domain, "Last reading is out of date");
        }
        const change = Math.round((1.0 - root.client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(root.domain, "Not changing brightness right now");
    }

    WeatherText {
        id: weatherText
    }

    AmigaStyle {
        id: tokens

        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: weatherText.summary(root.client)
    Accessible.description: effect

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "weather"

        AmigaWindow {
            id: panel

            theme: root.theme
            style: tokens
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the weather window", "Weather")
            contentHeight: effectText.y + effectText.height

            PixelSprite {
                id: sky

                source: Sprites.weatherAtlas()
                frame: root.client.hasWeatherReading
                    ? Sprites.conditionFrame(root.client.weatherCondition, root.client.weatherIsNight)
                    : Sprites.weatherFrame("unknown")
                colors: tokens.spriteColors
            }

            AmigaText {
                id: temp

                x: 20
                y: 0
                pitch: 2
                color: tokens.ink
                text: root.client.hasTemperature ? Math.round(root.client.temperatureC) + "°" : "--"
            }

            AmigaText {
                id: conditionText

                x: temp.x + temp.artWidth + 6
                y: 3
                color: tokens.ink
                text: root.condition
                maximumArtWidth: Math.max(8, panel.contentWidth - x)
            }

            AmigaText {
                id: effectText

                x: 0
                y: 16
                color: tokens.ink
                text: root.effect
                maximumArtWidth: panel.contentWidth
                maximumLines: 1
            }
        }
    }
}
