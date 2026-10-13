/*
 * Weather, Nothing: the reading as a dot-matrix pictogram and numeral, the
 * condition in small lower-case mono, the secondary readings as tiny columns,
 * and what the weather does to the targets as a row of dots. Lit dots are the
 * brightness the weather leaves, so a dimmed day shows unlit dots at the right.
 *
 *   ┌──────────────────────────────────────── 24 px radius, hairline ─┐
 *   │  [pictogram]  15°                                              │
 *   │               partly cloudy                                    │
 *   │  FEELS        RH          WIND          AQI                    │
 *   │  14°          62%         3.4 m/s       42                     │
 *   │  DIM −12%     ▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▪▫▫▫▫▫▫▫▫▫▫▫▫                    │
 *   └────────────────────────────────────────────────────────────────┘
 *
 * Shown only when weather is enabled. Without a reading the card says why.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

Rectangle {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    /*! A reading the daemon holds, and whether it is fresh enough to show its effect. */
    readonly property bool reading: client.hasWeatherReading
    readonly property bool fresh: reading && !client.weatherStale
    /*! Percent the weather takes off the targets; negative when it adds. */
    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)

    readonly property string stateNote: {
        if (client.hasWeatherReading) {
            return "";
        }
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

    readonly property string conditionText: {
        if (!root.reading) {
            return root.stateNote;
        }
        const label = weatherText.conditionLabel(root.client.weatherCondition)
            || i18nd(root.domain, "No weather reading");
        const text = root.client.weatherStale
            ? label + " · " + i18ndc(root.domain, "Tag: the weather reading is out of date", "Stale")
            : label;
        return text.toLowerCase();
    }

    readonly property string dimText: {
        if (root.change >= 2) {
            return i18ndc(root.domain, "Legend: weather is dimming the displays", "Dim") + " "
                + i18ndc(root.domain, "Percent points removed", "−%1%", root.change);
        }
        if (root.change <= -2) {
            return i18ndc(root.domain, "Legend: weather is brightening the displays", "Boost") + " "
                + i18ndc(root.domain, "Percent points added", "+%1%", -root.change);
        }
        return i18ndc(root.domain, "Legend: weather has no effect now", "No effect");
    }

    /*! The four secondary readings, in card order; "—" where the provider sends none. */
    readonly property var readings: {
        const d = root.client.weatherDetails;
        const none = "—";
        return [
            { k: i18ndc(root.domain, "Short legend: apparent temperature", "FEELS"),
              v: d.feelsLikeC !== undefined
                  ? i18ndc(root.domain, "Temperature in degrees, compact", "%1°", Math.round(d.feelsLikeC)) : none },
            { k: i18ndc(root.domain, "Short legend: relative humidity", "RH"),
              v: d.humidityPercent !== undefined
                  ? i18ndc(root.domain, "Percentage", "%1%", Math.round(d.humidityPercent)) : none },
            { k: i18ndc(root.domain, "Short legend: wind speed", "WIND"),
              v: d.windSpeedMps !== undefined
                  ? i18ndc(root.domain, "Wind speed in metres per second", "%1 m/s", d.windSpeedMps.toFixed(1)) : none },
            { k: i18ndc(root.domain, "Short legend: US EPA air quality index", "AQI"),
              v: d.airQualityIndex !== undefined ? String(Math.round(d.airQualityIndex)) : none },
        ];
    }

    WeatherText {
        id: weatherText
    }

    implicitHeight: content.implicitHeight + 2 * root.style.pad
    radius: root.style.radius
    antialiasing: true
    color: root.style.surface
    border.width: root.style.hairline
    border.color: root.style.outline

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(root.domain, "Weather")
    Accessible.description: root.conditionText

    ColumnLayout {
        id: content

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap

        // --- the reading: pictogram, numeral and condition ---------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.pad

            DotIcon {
                // Twelve dots tall: about the height of the temperature and condition beside it.
                Layout.alignment: Qt.AlignTop
                pitch: root.style.pitch
                condition: root.reading ? root.client.weatherCondition : ""
                night: root.reading && root.client.weatherIsNight
                litColor: root.style.ink
                accentColor: root.style.accent
                dimColor: root.style.dotGhost
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: root.style.gap * 0.5

                DotText {
                    visible: root.reading && root.client.hasTemperature
                    text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                 Math.round(root.client.temperatureC))
                    face: "small"
                    size: 1
                    letterSpacing: 1
                    pitch: root.style.pitchBig
                    litColor: root.style.ink
                    accentColor: root.style.accent
                    dimColor: root.style.textGhost
                }

                Text {
                    Layout.fillWidth: true
                    text: root.conditionText
                    textFormat: Text.PlainText
                    font: root.style.plain
                    color: root.reading ? root.style.ink : root.style.muted
                    elide: Text.ElideRight
                }
            }
        }

        // --- the effect on the targets: a row of dots, lit by what the weather leaves ---
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: root.style.gap * 0.5
            visible: root.fresh
            spacing: root.style.gap

            Text {
                text: root.dimText
                textFormat: Text.PlainText
                font: root.style.label
                color: root.style.muted
            }

            Item {
                id: track

                Layout.fillWidth: true
                implicitHeight: dimBar.implicitHeight

                DotBar {
                    id: dimBar

                    anchors.left: parent.left
                    anchors.right: parent.right
                    cols: Math.max(8, Math.floor(track.width / root.style.pitch))
                    rows: 1
                    pitch: root.style.pitch
                    value: Math.max(0, Math.min(1, root.client.weatherMultiplier))
                    thumbAccent: false
                    litColor: root.style.ink
                    dimColor: root.style.dotGhost
                }
            }
        }

        // --- the secondary readings, as tiny columns ---------------------------------------
        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: root.style.gap * 0.5
            visible: root.reading
            spacing: root.style.gap

            Repeater {
                model: root.readings

                delegate: ColumnLayout {
                    id: column

                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 0

                    Text {
                        Layout.fillWidth: true
                        text: column.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.label
                        color: root.style.muted
                        elide: Text.ElideRight
                    }
                    Text {
                        Layout.fillWidth: true
                        text: column.modelData.v
                        textFormat: Text.PlainText
                        font: root.style.plain
                        color: root.style.ink
                        elide: Text.ElideRight
                    }
                }
            }
        }
    }
}
