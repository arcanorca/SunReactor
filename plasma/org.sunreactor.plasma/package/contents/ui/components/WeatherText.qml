/*
 * Translated names for the daemon's weather condition tokens. Kept in one
 * place so the popup and the panel tooltip cannot drift apart.
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
