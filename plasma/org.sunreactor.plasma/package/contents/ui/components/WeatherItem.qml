/*
 * Weather as it concerns brightness: what it is outside, and what that does to
 * the targets. The daemon's bounded multiplier is stated in words, because
 * "0.88x" is not something to put in front of a user.
 */

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../Icons.js" as Icons

PlasmaComponents3.ItemDelegate {
    id: root

    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    readonly property string summary: {
        const text = weatherText.summary(client);
        return text.length > 0 ? text : i18nd(root.domain, "No weather reading");
    }

    readonly property string effect: {
        if (!client.hasWeatherReading) {
            switch (client.weatherState) {
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

    /*! Readings that do not earn a line of their own, for the tooltip. */
    readonly property string extras: {
        const details = client.weatherDetails;
        const parts = [];
        if (details.feelsLikeC !== undefined) {
            parts.push(i18ndc(root.domain, "Apparent temperature", "Feels like %1 °C",
                              Math.round(details.feelsLikeC)));
        }
        if (details.humidityPercent !== undefined) {
            parts.push(i18ndc(root.domain, "Relative humidity", "Humidity %1%",
                              Math.round(details.humidityPercent)));
        }
        if (details.windSpeedMps !== undefined) {
            parts.push(i18ndc(root.domain, "Wind speed in metres per second", "Wind %1 m/s",
                              details.windSpeedMps.toFixed(1)));
        }
        if (details.airQualityIndex !== undefined) {
            parts.push(i18ndc(root.domain, "US EPA air quality index", "Air quality index %1",
                              Math.round(details.airQualityIndex)));
        }
        return parts.join("\n");
    }

    Layout.fillWidth: true

    background.visible: false
    hoverEnabled: extras.length > 0
    Accessible.ignored: true

    QQC2.ToolTip.visible: hovered && root.extras.length > 0
    QQC2.ToolTip.text: root.extras

    WeatherText {
        id: weatherText
    }

    contentItem: RowLayout {
        spacing: Kirigami.Units.gridUnit

        Kirigami.Icon {
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            roundToIconSize: false
            source: root.client.hasWeatherReading
                ? Icons.conditionArt(root.client.weatherCondition, root.client.weatherIsNight)
                : Icons.art("unknown")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.summary
                textFormat: Text.PlainText
                elide: Text.ElideRight
            }

            PlasmaExtras.DescriptiveLabel {
                Layout.fillWidth: true
                text: root.effect
                textFormat: Text.PlainText
                elide: Text.ElideRight
            }
        }
    }
}
