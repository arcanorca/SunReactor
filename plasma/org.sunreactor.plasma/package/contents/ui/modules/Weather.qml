/*
 * Weather module: what it is outside, and what that does to the targets. The
 * daemon's bounded multiplier is stated in words, because "0.88x" is not
 * something to put in front of a user. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../components"
import "../Format.js" as Format

PlasmaComponents3.ItemDelegate {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    readonly property string summary: {
        const text = weatherText.summary(client);
        return text.length > 0 ? text : i18nd(root.domain, "No weather reading");
    }

    readonly property string effect: weatherText.effect(client)

    /*! Readings that do not earn a line of their own, for the tooltip. */
    readonly property string extras: {
        const details = client.weatherDetails;
        const parts = [];
        if (details.feelsLikeC !== undefined) {
            const feels = Format.tempRounded(details.feelsLikeC, client.isFahrenheit);
            parts.push(client.isFahrenheit
                ? i18ndc(root.domain, "Apparent temperature", "Feels like %1 °F", feels)
                : i18ndc(root.domain, "Apparent temperature", "Feels like %1 °C", feels));
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
        spacing: root.theme.gridUnit

        WeatherIcon {
            Layout.preferredWidth: Math.round(Kirigami.Units.iconSizes.medium * root.theme.scaleFactor)
            Layout.preferredHeight: Layout.preferredWidth
            theme: root.theme
            condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
            night: root.client.hasWeatherReading && root.client.weatherIsNight
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                visible: root.client.locationCity.length > 0
                text: root.client.locationCity
                textFormat: Text.PlainText
                font.family: root.theme.bodyFont
                font.pixelSize: root.theme.fontSizeSmall
                font.weight: Font.DemiBold
                color: root.theme.textMuted
                elide: Text.ElideRight
            }

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.summary
                textFormat: Text.PlainText
                font.family: root.theme.bodyFont
                font.pixelSize: root.theme.fontSizeBody
                color: root.theme.fg
                elide: Text.ElideRight
            }

            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.effect
                textFormat: Text.PlainText
                font.family: root.theme.bodyFont
                font.pixelSize: root.theme.fontSizeSmall
                color: root.theme.textMuted
                elide: Text.ElideRight
            }
        }
    }
}
