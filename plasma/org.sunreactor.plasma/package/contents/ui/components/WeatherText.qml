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

    /*! What the weather does to the brightness right now, or why it cannot say. */
    function effect(client) {
        if (!client.hasWeatherReading) {
            return stateNote(client.weatherState);
        }
        if (client.weatherStale) {
            return i18nd(root.domain, "Last reading is out of date");
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(root.domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(root.domain, "Not changing brightness right now");
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
