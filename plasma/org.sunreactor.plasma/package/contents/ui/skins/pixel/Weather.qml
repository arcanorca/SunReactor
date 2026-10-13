/*
 * Pixel Weather: the sky sprite (indexed atlas recoloured by the theme) at
 * 1:1 art scale, the temperature in the hardware's digits, the condition and
 * what the daemon's bounded multiplier does to the targets, in words.
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    readonly property string condition: {
        const text = weatherText.conditionLabel(client.weatherCondition);
        return client.hasWeatherReading && text.length > 0 ? text : i18nd(domain, "No weather reading");
    }
    readonly property string effect: {
        if (!client.hasWeatherReading) {
            switch (client.weatherState) {
            case "no_api_key":
                return i18nd(domain, "No API key is configured");
            case "loading":
                return i18nd(domain, "Looking up the current conditions…");
            case "unauthorized":
                return i18nd(domain, "The weather provider rejected the API key");
            case "rate_limited":
                return i18nd(domain, "The weather provider is rate limiting requests");
            case "network_error":
                return i18nd(domain, "The weather provider could not be reached");
            default:
                return i18nd(domain, "No reading yet");
            }
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
    Accessible.description: effect

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "weather"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the weather panel", "Weather")
            contentHeight: Math.max(sprite.height, temp.height, conditionText.height + 2 + effectText.height)

            // The 16 px sky sprite, or the vector glyph (solid, by default) on the
            // same 16 px footprint when the weather icon style is not "pixel".
            PixelSprite {
                id: sprite
                visible: root.theme.glyphStyle === "pixel"
                source: Sprites.weatherAtlas()
                frame: root.client.hasWeatherReading
                    ? Sprites.conditionFrame(root.client.weatherCondition, root.client.weatherIsNight)
                    : Sprites.weatherFrame("unknown")
                colors: pstyle.spriteColors
            }
            WeatherGlyph {
                visible: root.theme.glyphStyle !== "pixel"
                size: 16
                condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
                night: root.client.hasWeatherReading && root.client.weatherIsNight
                glyphStyle: root.theme.glyphStyle
                palette: root.theme.glyphColours(pstyle.paper, pstyle.ink)
            }
            PixelDigits {
                id: temp
                theme: root.theme
                style: pstyle
                x: 20
                y: Math.max(0, Math.floor((sprite.height - height) / 2))
                size: pstyle.kind === "c64" ? 1 : 2
                text: root.client.hasTemperature ? Math.round(root.client.temperatureC) + "°" : "--"
            }
            PixelLabel {
                id: conditionText
                theme: root.theme
                x: temp.x + temp.width + 6
                text: root.condition
                face: pstyle.bodyFace
                color: pstyle.ink
                maximumArtWidth: Math.max(8, panel.contentWidth - x)
            }
            PixelLabel {
                id: effectText
                theme: root.theme
                x: conditionText.x
                y: conditionText.height + 2
                text: root.effect
                face: pstyle.bodyFace
                color: pstyle.dim
                stipple: pstyle.stippleDim
                maximumArtWidth: Math.max(8, panel.contentWidth - x)
                maximumLines: 2
            }
        }
    }
}
