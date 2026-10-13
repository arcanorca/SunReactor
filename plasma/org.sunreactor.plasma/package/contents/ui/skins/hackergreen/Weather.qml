/*
 * hackergreen weather: the current conditions as an ASCII sky beside the
 * readings, the terminal skin's list in phosphor.
 *
 *   == WEATHER ============================================  [ OK ]
 *    \ | /   Partly cloudy  15.2°C
 *   -- O --  Effect   Dimming displays by 12%
 *    / | \   Feels    13.9°C     Humidity  62%
 *            Wind     3.4 m/s    Air AQI   42
 *
 * Only readings the daemon sent are listed; nothing is invented. The art is
 * drawn from Hg.js, one token per condition, moon at night.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../terminal"
import "../terminal/Term.js" as Term
import "Hg.js" as Hg

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    WeatherText {
        id: weatherText
    }

    TermMetrics {
        id: term

        theme: root.theme
    }
    HgCrt {
        theme: root.theme
    }

    readonly property string effect: {
        if (!client.hasWeatherReading) {
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
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(domain, "Not changing brightness");
    }

    /*! [label, value] pairs the daemon actually reported. */
    readonly property var readings: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push([i18ndc(domain, "Apparent temperature", "Feels"), d.feelsLikeC.toFixed(1) + "°C"]);
        }
        if (d.humidityPercent !== undefined) {
            out.push([i18ndc(domain, "Relative humidity", "Humidity"), Math.round(d.humidityPercent) + "%"]);
        }
        if (client.cloudCoverPercent >= 0) {
            out.push([i18ndc(domain, "Share of the sky covered by cloud", "Cloud"), client.cloudCoverPercent + "%"]);
        }
        if (d.windSpeedMps !== undefined) {
            out.push([i18ndc(domain, "Wind speed", "Wind"), d.windSpeedMps.toFixed(1) + " m/s"]);
        }
        if (d.airQualityIndex !== undefined) {
            out.push([i18ndc(domain, "US EPA air quality index", "AQI"), String(Math.round(d.airQualityIndex))]);
        }
        return out;
    }

    readonly property bool night: client.sunriseEpochS > 0 && client.sunsetEpochS > 0
        && (client.nowEpochS < client.sunriseEpochS || client.nowEpochS > client.sunsetEpochS)
    readonly property var artRows: Hg.art(client.weatherCondition, night)
    readonly property color artColor: {
        const c = client.weatherCondition;
        if (c === "clear") {
            return root.theme.accent;
        }
        if (c === "mist" || c === "fog" || c === "atmospheric") {
            return root.theme.textMuted;
        }
        return root.theme.fg;
    }

    readonly property int artCells: 7
    readonly property int textX: 1 + artCells + 1
    readonly property int textCells: Math.max(0, term.cols(root.width) - textX - 1)
    /*! A reading's label, its gap and its value (seven cells at most). */
    readonly property int labelCells: 9
    /*! Two readings side by side when each fits: the left one, then the right one. */
    readonly property bool twoColumns: textCells >= 2 * (labelCells + 7)
    readonly property int half: Math.floor(textCells / 2)
    readonly property int readingRows: twoColumns ? Math.ceil(readings.length / 2) : readings.length
    readonly property int textRows: 2 + readingRows

    implicitHeight: section.implicitHeight

    HgSection {
        id: section

        tm: term
        theme: root.theme
        width: root.width
        title: i18ndc(root.domain, "Section title of the weather", "WEATHER")
        metaRuns: {
            const t = root.theme;
            if (!root.client.hasWeatherReading) {
                return [Hg.flag(t, "dim", i18ndc(root.domain, "Weather reading state", "NO READING"))];
            }
            if (root.client.weatherStale) {
                return [Hg.flag(t, "warn", i18ndc(root.domain, "Weather reading state", "STALE"))];
            }
            return [Hg.flag(t, "ok", i18ndc(root.domain, "Weather reading state", "OK"))];
        }
        bodyHeight: term.cy(Math.max(3, root.textRows))
        glowing: true

        // The sky, three rows of seven cells.
        Repeater {
            model: root.artRows

            delegate: TermText {
                required property string modelData
                required property int index

                tm: term
                x: term.cx(1)
                y: term.cy(index)
                value: modelData
                strong: true
                color: root.artColor
            }
        }

        TermRuns {
            tm: term
            x: term.cx(root.textX)
            cells: root.textCells
            runs: {
                const condition = weatherText.conditionLabel(root.client.weatherCondition);
                const out = [];
                if (condition.length > 0) {
                    out.push({ t: condition, c: root.theme.fg, b: true });
                }
                if (root.client.hasTemperature) {
                    if (out.length > 0) {
                        out.push({ t: "  ", c: root.theme.textMuted });
                    }
                    out.push({ t: root.client.temperatureC.toFixed(1) + "°C", c: root.theme.accent, b: true });
                }
                if (out.length === 0) {
                    out.push({ t: i18nd(root.domain, "No weather reading"), c: root.theme.textMuted });
                }
                return out;
            }
        }

        // The effect on the targets: the sentence says what it is, so it has no label.
        TermText {
            tm: term
            x: term.cx(root.textX)
            y: term.cy(1)
            value: root.effect
            cells: Math.max(0, root.textCells)
            color: root.client.hasWeatherReading ? root.theme.fg : root.theme.textMuted
        }

        Repeater {
            model: root.readings

            delegate: TermRuns {
                required property var modelData
                required property int index

                tm: term
                x: term.cx(root.textX + (root.twoColumns && index % 2 === 1 ? root.half : 0))
                y: term.cy(2 + (root.twoColumns ? Math.floor(index / 2) : index))
                cells: root.twoColumns ? (index % 2 === 1 ? root.textCells - root.half : root.half - 1) : root.textCells
                runs: [{ t: Term.padEnd(modelData[0], root.labelCells), c: root.theme.textMuted },
                       { t: modelData[1], c: root.theme.fg }]
            }
        }
    }
}
