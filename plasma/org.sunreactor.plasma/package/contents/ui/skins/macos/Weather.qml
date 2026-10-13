/*
 * Weather, macos: the reading the way Apple's Weather widget sets it. The
 * temperature in large light figures with the condition and the day's high and
 * low beside it, the sky on the right, then four tiles: feels like, humidity,
 * wind, and how much the weather dims the displays. Shown only when weather is
 * enabled.
 *
 *   18°                                          ☁
 *   Partly cloudy                         H 21°  L 9°
 *   ╭─────────────────╮ ╭─────────────────╮
 *   │ Feels like  14° │ │ Humidity    62% │
 *   ╰─────────────────╯ ╰─────────────────╯
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property bool available: client.isConnected && client.weatherEnabled
    readonly property bool hasReading: client.hasWeatherReading
    /*! Percent the weather takes off the brightness (positive means dimmer). */
    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)

    readonly property string stateNote: {
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

    /*! Highest and lowest temperature of the reading and the next 24 hours; null when unknown. */
    readonly property var span: {
        let high = null;
        let low = null;
        const take = v => {
            high = high === null ? v : Math.max(high, v);
            low = low === null ? v : Math.min(low, v);
        };
        if (client.hasTemperature) {
            take(client.temperatureC);
        }
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS && point.epochS <= client.nowEpochS + 86400
                    && typeof point.temperatureC === "number") {
                take(point.temperatureC);
            }
        }
        return high === null ? null : { high: Math.round(high), low: Math.round(low) };
    }

    /*! The four tiles: { k: label, v: value, warn }. */
    readonly property var tiles: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push({ k: i18ndc(domain, "Weather tile: apparent temperature", "Feels like"),
                       v: i18ndc(domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)),
                       warn: false });
        }
        if (d.humidityPercent !== undefined) {
            out.push({ k: i18ndc(domain, "Weather tile: relative humidity", "Humidity"),
                       v: i18ndc(domain, "Percentage", "%1%", Math.round(d.humidityPercent)),
                       warn: false });
        }
        if (d.windSpeedMps !== undefined) {
            out.push({ k: i18ndc(domain, "Weather tile: wind speed", "Wind"),
                       v: i18ndc(domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)),
                       warn: false });
        }
        out.push({ k: i18ndc(domain, "Weather tile: how much the weather dims the displays", "Dimming"),
                   v: root.change >= 1 ? "−" + root.change + "%" : i18ndc(domain, "No dimming", "None"),
                   warn: root.change >= 1 });
        return out.slice(0, 4);
    }

    WeatherText {
        id: weatherText
    }

    spacing: root.style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the weather reading", "Weather")

    // --- the reading: the temperature and its condition, the sky and the day's range on the right
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                Layout.fillWidth: true
                text: root.client.hasTemperature ? i18ndc(root.domain, "Temperature in degrees, compact",
                                                          "%1°", Math.round(root.client.temperatureC)) : "—"
                textFormat: Text.PlainText
                font: root.style.temperature
                color: root.style.ink
                lineHeight: 0.75
                lineHeightMode: Text.ProportionalHeight
            }

            Text {
                Layout.fillWidth: true
                text: root.hasReading
                    ? (weatherText.conditionLabel(root.client.weatherCondition) || i18nd(root.domain, "No reading yet"))
                    : root.stateNote
                textFormat: Text.PlainText
                font: root.style.body
                color: root.hasReading ? root.style.ink : root.style.inkSecondary
                elide: Text.ElideRight
            }
        }

        ColumnLayout {
            Layout.alignment: Qt.AlignTop
            spacing: 0

            WeatherGlyph {
                Layout.alignment: Qt.AlignRight
                visible: root.hasReading
                size: root.style.snap(34)
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
                glyphStyle: "solid"
                palette: root.theme.glyphPalette
            }

            Text {
                Layout.alignment: Qt.AlignRight
                visible: text.length > 0
                text: root.client.weatherStale && root.hasReading
                    ? i18nd(root.domain, "Old reading")
                    : (root.span !== null
                        ? i18ndc(root.domain, "Today's high and low temperature", "H %1°  L %2°",
                                 root.span.high, root.span.low)
                        : "")
                textFormat: Text.PlainText
                font: root.style.caption
                color: root.style.inkSecondary
            }
        }
    }

    // --- the tiles: a 2 x 2 grid of rounded panels, label and value on one line ----
    GridLayout {
        Layout.fillWidth: true
        columns: 2
        columnSpacing: root.style.gap
        rowSpacing: root.style.snap(4)

        Repeater {
            model: root.tiles

            delegate: Surface {
                required property var modelData

                Layout.fillWidth: true
                style: root.style
                padding: root.style.snap(5)

                RowLayout {
                    Layout.fillWidth: true
                    spacing: root.style.gap

                    Text {
                        Layout.fillWidth: true
                        text: modelData.k
                        textFormat: Text.PlainText
                        font: root.style.caption
                        color: root.style.inkSecondary
                        elide: Text.ElideRight
                    }
                    Text {
                        text: modelData.v
                        textFormat: Text.PlainText
                        font: root.style.readout
                        color: modelData.warn ? root.style.warning : root.style.ink
                    }
                }
            }
        }
    }
}
