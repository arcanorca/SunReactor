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
    /*! The status phrase in the longest wording that fits the line: the forms
        are measured with AmigaText probes (below), never elided. */
    readonly property var effectForms: {
        if (!root.client.hasWeatherReading) {
            switch (root.client.weatherState) {
            case "no_api_key":
                return [i18nd(root.domain, "No API key is configured"), i18nd(root.domain, "No API key")];
            case "loading":
                return [i18nd(root.domain, "Looking up the current conditions…"), i18nd(root.domain, "Looking up…")];
            case "unauthorized":
                return [i18nd(root.domain, "The weather provider rejected the API key"), i18nd(root.domain, "API key rejected")];
            case "rate_limited":
                return [i18nd(root.domain, "The weather provider is rate limiting requests"), i18nd(root.domain, "Rate limited")];
            case "network_error":
                return [i18nd(root.domain, "The weather provider could not be reached"), i18nd(root.domain, "Provider unreachable")];
            default:
                return [i18nd(root.domain, "No reading yet")];
            }
        }
        if (root.client.weatherStale) {
            return [i18nd(root.domain, "Last reading is out of date"), i18nd(root.domain, "Reading out of date")];
        }
        const change = Math.round((1.0 - root.client.weatherMultiplier) * 100);
        if (change >= 2) {
            return [i18ndc(root.domain, "Placeholder is a percentage", "Dimming displays by %1%", change),
                    i18ndc(root.domain, "Placeholder is a percentage", "Dimming by %1%", change),
                    i18ndc(root.domain, "Placeholder is a percentage", "Dim %1%", change)];
        }
        if (change <= -2) {
            return [i18ndc(root.domain, "Placeholder is a percentage", "Brightening displays by %1%", -change),
                    i18ndc(root.domain, "Placeholder is a percentage", "Brightening by %1%", -change),
                    i18ndc(root.domain, "Placeholder is a percentage", "Brighter %1%", -change)];
        }
        return [i18nd(root.domain, "Not changing brightness right now"), i18nd(root.domain, "No change")];
    }
    readonly property string effect: effectForms[0]

    /*! The longest form whose art width fits `limit` (art px); the probes
        below measure each form with no limit. */
    function effectFor(limit) {
        // Read so the binding follows the probes being created (itemAt is not tracked).
        if (effectProbes.count < effectForms.length) {
            return effectForms[0];
        }
        for (let i = 0; i < effectForms.length; ++i) {
            const probe = effectProbes.itemAt(i);
            if (probe && probe.artWidth <= limit) {
                return effectForms[i];
            }
        }
        return effectForms[effectForms.length - 1];
    }
    Repeater {
        id: effectProbes

        model: root.effectForms

        AmigaText {
            required property string modelData

            visible: false
            text: modelData
            maximumArtWidth: 0
        }
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
                // A long condition takes a second line (by words), never an ellipsis.
                maximumArtWidth: Math.max(8, panel.contentWidth - x)
                maximumLines: 2
            }

            AmigaText {
                id: effectText

                // Below the sky and the figure, and below a condition on two lines.
                x: 0
                y: Math.max(16, conditionText.y + conditionText.artHeight + 3)
                color: tokens.ink
                text: root.effectFor(panel.contentWidth)
                maximumArtWidth: panel.contentWidth
                maximumLines: 1
            }
        }
    }
}
