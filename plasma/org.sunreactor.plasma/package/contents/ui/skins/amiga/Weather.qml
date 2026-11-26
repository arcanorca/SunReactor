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
import "../../components/weathericons"
import "../../modules"
import "../../components"
import "../../Format.js" as Format
import "../pixel/Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    readonly property string condition: {
        const text = weatherText.conditionLabel(root.client.weatherCondition);
        return root.client.hasWeatherReading && text.length > 0 ? text : i18nd(root.domain, "No weather reading");
    }
    /*! The status phrase in the longest wording that fits the line: the forms
        are measured with AmigaText probes (below), never elided. */
    readonly property var effectForms: weatherText.effectForms(root.client)
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
            if (probe && probe.artWidth <= limit) { // qmllint disable missing-property
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

            // The atlas sprite, or the chosen Weather icons style in its place.
            ThemedWeatherIcon {
                theme: root.theme
                size: 16
                condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
                night: root.client.hasWeatherReading && root.client.weatherIsNight

                PixelSprite {
                    id: sky

                    source: Sprites.weatherAtlas()
                    frame: root.client.hasWeatherReading
                        ? Sprites.conditionFrame(root.client.weatherCondition, root.client.weatherIsNight)
                        : Sprites.weatherFrame("unknown")
                    colors: tokens.spriteColors
                }
            }

            AmigaText {
                id: temp

                x: 20
                y: 0
                pitch: 2
                color: tokens.ink
                text: root.client.hasTemperature ? Format.tempRounded(root.client.temperatureC, root.client.isFahrenheit) + "°" : "--"
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
