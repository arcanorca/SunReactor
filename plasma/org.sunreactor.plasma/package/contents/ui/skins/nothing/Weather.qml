/*
 * Weather, Nothing: the reading as a dot-matrix pictogram beside its temperature
 * in dot numerals, the condition and what the weather does to the targets in one
 * small lower-case line, and the secondary readings as a 2 x 2 grid of label and
 * value.
 *
 *   ┌──────────────────────────────────────── 24 px radius, hairline ─┐
 *   │  [pictogram]  15°                                              │
 *   │               partly cloudy · dim −12%                         │
 *   │  FEELS  14°               RH  62%                              │
 *   │  WIND   3.4 m/s           AQI  42                              │
 *   └────────────────────────────────────────────────────────────────┘
 *
 * Shown only when weather is enabled. Without a reading the card says why.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../components"
import "../../Format.js" as Format

Rectangle {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    /*! A reading the daemon holds, and whether it is fresh enough to show its effect. */
    readonly property bool reading: client.hasWeatherReading
    readonly property bool fresh: reading && !client.weatherStale
    /*! Percent the weather takes off the targets; negative when it adds. */
    readonly property int change: Math.round((1.0 - client.weatherMultiplier) * 100)

    readonly property string stateNote: client.hasWeatherReading ? "" : weatherText.stateNote(client.weatherState)

    /*! The condition, the staleness and the weather's effect, in one lower-case line. */
    readonly property string conditionText: {
        if (!root.reading) {
            return root.stateNote;
        }
        const label = weatherText.conditionLabel(root.client.weatherCondition)
            || i18nd(root.domain, "No weather reading");
        const parts = [root.client.weatherStale
            ? label + " · " + i18ndc(root.domain, "Tag: the weather reading is out of date", "Stale")
            : label];
        if (root.fresh) {
            parts.push(root.dimText);
        }
        return parts.join(" · ").toLowerCase();
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
                  ? i18ndc(root.domain, "Temperature in degrees, compact", "%1°", Format.tempRounded(d.feelsLikeC, root.client.isFahrenheit)) : none },
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

        // --- the reading: pictogram, temperature and condition --------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.pad

            ThemedWeatherIcon {
                Layout.alignment: Qt.AlignTop
                theme: root.theme
                size: root.style.snap(root.style.unit * 3.0)
                condition: root.reading ? root.client.weatherCondition : ""
                night: root.reading && root.client.weatherIsNight

                NothingIcon {
                    anchors.centerIn: parent
                    theme: root.theme
                    surface: root.style.surface
                    ink: root.style.ink
                    size: root.style.snap(root.style.unit * 3.0)
                    condition: root.reading ? root.client.weatherCondition : ""
                    night: root.reading && root.client.weatherIsNight
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.alignment: Qt.AlignVCenter
                spacing: root.style.gap * 0.5

                Text {
                    Layout.fillWidth: true
                    visible: root.client.locationCity.length > 0
                    text: root.client.locationCity.toUpperCase()
                    textFormat: Text.PlainText
                    font: root.style.label
                    color: root.style.muted
                    elide: Text.ElideRight
                }

                Text {
                    visible: root.reading && root.client.hasTemperature
                    text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                 Format.tempRounded(root.client.temperatureC, root.client.isFahrenheit))
                    textFormat: Text.PlainText
                    font: root.style.dotHero
                    color: root.style.ink
                }

                Text {
                    Layout.fillWidth: true
                    text: root.conditionText
                    textFormat: Text.PlainText
                    font: root.style.sentence
                    color: root.reading ? root.style.ink : root.style.muted
                    wrapMode: Text.WordWrap
                    maximumLineCount: 2
                    elide: Text.ElideRight
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            visible: root.reading
            implicitHeight: root.style.hairline
            color: root.style.outline
        }

        // --- the secondary readings: a 2 x 2 grid, each a label and its value --------------
        GridLayout {
            Layout.fillWidth: true
            visible: root.reading
            columns: 2
            columnSpacing: root.style.gap * 2
            rowSpacing: root.style.gap * 0.5

            Repeater {
                model: root.readings

                delegate: RowLayout {
                    id: cell

                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: root.style.gap * 0.6

                    Text {
                        Layout.alignment: Qt.AlignBaseline
                        text: cell.modelData.k
                        textFormat: Text.PlainText
                        font: root.style.label
                        color: root.style.muted
                    }
                    Text {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignBaseline
                        text: cell.modelData.v
                        textFormat: Text.PlainText
                        font: root.style.figure
                        color: root.style.ink
                        elide: Text.ElideRight
                    }
                }
            }
        }
    }
}
