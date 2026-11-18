/*
 * Translated names for the daemon's weather condition tokens, and the sentences
 * the weather modules share (why there is no reading, what the weather does to
 * the brightness). Kept in one place so the popup, its skins and the panel
 * tooltip cannot drift apart.
 */

import QtQuick

QtObject {
    id: root

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    function conditionLabel(condition) {
        switch (condition) {
        case "clear":
            return i18ndc(root.domain, "Weather condition", "Clear");
        case "partly_cloudy":
            return i18ndc(root.domain, "Weather condition", "Partly cloudy");
        case "cloudy":
            return i18ndc(root.domain, "Weather condition", "Cloudy");
        case "drizzle":
            return i18ndc(root.domain, "Weather condition", "Drizzle");
        case "rain":
            return i18ndc(root.domain, "Weather condition", "Rain");
        case "heavy_rain":
            return i18ndc(root.domain, "Weather condition", "Heavy rain");
        case "thunderstorm":
            return i18ndc(root.domain, "Weather condition", "Thunderstorm");
        case "snow":
            return i18ndc(root.domain, "Weather condition", "Snow");
        case "mist":
            return i18ndc(root.domain, "Weather condition", "Mist");
        case "fog":
            return i18ndc(root.domain, "Weather condition", "Fog");
        case "atmospheric":
            return i18ndc(root.domain, "Weather condition", "Hazy");
        default:
            return "";
        }
    }

    function shortConditionLabel(condition, caps) {
        let label = "";
        switch (condition) {
        case "clear":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "CLEAR")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Clear");
            break;
        case "partly_cloudy":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "P.CLDY")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "P.cloudy");
            break;
        case "cloudy":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "CLOUDY")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Cloudy");
            break;
        case "drizzle":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "DRIZZLE")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Drizzle");
            break;
        case "rain":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "RAIN")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Rain");
            break;
        case "heavy_rain":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "H.RAIN")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "H.rain");
            break;
        case "thunderstorm":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "STORM")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Storm");
            break;
        case "snow":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "SNOW")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Snow");
            break;
        case "mist":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "MIST")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Mist");
            break;
        case "fog":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "FOG")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Fog");
            break;
        case "atmospheric":
            label = caps
                ? i18ndc(root.domain, "Short weather condition (max ~7 letters)", "HAZE")
                : i18ndc(root.domain, "Short weather condition (max ~7 letters)", "Haze");
            break;
        default:
            label = "";
            break;
        }
        return label;
    }

    /*! Why there is no weather reading, from the daemon's weather state token. */
    function stateNote(state) {
        switch (state) {
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

    /*! Progressively shorter phrases describing what the weather does right now.
        Pass `includeStale = false` on skins that already show stale in a status badge. */
    function effectForms(client, includeStale) {
        if (!client.hasWeatherReading) {
            switch (client.weatherState) {
            case "no_api_key":
                return [i18nd(root.domain, "No API key is configured"), i18nd(root.domain, "No API key")];
            case "loading":
                return [i18nd(root.domain, "Looking up the current conditions…"),
                        i18nd(root.domain, "Looking up conditions…"),
                        i18nd(root.domain, "Looking up…")];
            case "unauthorized":
                return [i18nd(root.domain, "The weather provider rejected the API key"),
                        i18nd(root.domain, "API key rejected")];
            case "rate_limited":
                return [i18nd(root.domain, "The weather provider is rate limiting requests"),
                        i18nd(root.domain, "Rate limited")];
            case "network_error":
                return [i18nd(root.domain, "The weather provider could not be reached"),
                        i18nd(root.domain, "Provider unreachable")];
            default:
                return [i18nd(root.domain, "No reading yet")];
            }
        }
        if (includeStale !== false && client.weatherStale) {
            return [i18nd(root.domain, "Last reading is out of date"),
                    i18nd(root.domain, "Reading out of date")];
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
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

    /*! What the weather does to the brightness right now, or why it cannot say. */
    function effect(client, includeStale) {
        return effectForms(client, includeStale)[0];
    }

    /*! Secondary weather readings formatted as [{ k, v }] pairs in "full", "compact", or "caps" label mode. */
    function detailsList(details, mode) {
        const d = details || ({});
        const out = [];
        const isCaps = mode === "caps";
        const isCompact = mode === "compact";
        if (d.feelsLikeC !== undefined) {
            const k = isCaps ? i18ndc(root.domain, "Short legend: apparent temperature", "FEELS")
                : (isCompact ? i18ndc(root.domain, "Short legend: apparent temperature", "Feels")
                             : i18ndc(root.domain, "Weather details: apparent temperature", "Feels like"));
            out.push({ k: k, v: i18ndc(root.domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) });
        }
        if (d.humidityPercent !== undefined) {
            const k = isCaps ? i18ndc(root.domain, "Short legend: relative humidity", "RH")
                : (isCompact ? i18ndc(root.domain, "Short legend: relative humidity", "RH")
                             : i18ndc(root.domain, "Weather details: relative humidity", "Humidity"));
            out.push({ k: k, v: i18ndc(root.domain, "Percentage", "%1%", Math.round(d.humidityPercent)) });
        }
        if (d.windSpeedMps !== undefined) {
            const k = isCaps ? i18ndc(root.domain, "Short legend: wind speed", "WIND")
                : (isCompact ? i18ndc(root.domain, "Short legend: wind speed", "Wind")
                             : i18ndc(root.domain, "Weather details: wind speed", "Wind"));
            out.push({ k: k, v: i18ndc(root.domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) });
        }
        if (d.airQualityIndex !== undefined) {
            const k = isCaps ? i18ndc(root.domain, "Short legend: US EPA air quality index", "AQI")
                : (isCompact ? i18ndc(root.domain, "Short legend: US EPA air quality index", "AQI")
                             : i18ndc(root.domain, "Weather details: US EPA air quality index", "Air quality"));
            out.push({ k: k, v: String(Math.round(d.airQualityIndex)) });
        }
        return out;
    }

    function temperatureLabel(celsius) {
        return i18ndc(root.domain, "Temperature in degrees Celsius", "%1 °C", Math.round(celsius));
    }

    /*! "21 °C · Partly cloudy", or whichever half of it is known. */
    function summary(client) {
        const condition = conditionLabel(client.weatherCondition);
        if (!client.hasTemperature) {
            return condition;
        }
        const temperature = temperatureLabel(client.temperatureC);
        return condition.length > 0
            ? i18ndc(root.domain, "Temperature, then sky condition", "%1 · %2", temperature, condition)
            : temperature;
    }
}
