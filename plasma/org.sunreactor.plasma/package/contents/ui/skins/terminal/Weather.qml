/*
 * Terminal skin of the weather module: the TUI's "Current conditions" panel
 * (src/tui/ui/weather.rs) as data lines.
 *
 *     ╭ Current conditions ────────── ● Up to date ╮
 *     │ Partly cloudy · 15.2°C                     │
 *     │ Effect      Dimming displays by 12%        │
 *     │ Feels like  13.9°C     Humidity    62%     │
 *     │ Wind        3.4 m/s    Air quality 42      │
 *     ╰────────────────────────────────────────────╯
 *
 * The daemon's bounded multiplier is said in words. Only readings the daemon
 * sent are listed; nothing is invented.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Term.js" as Term

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
            out.push([i18ndc(domain, "Apparent temperature", "Feels like"), d.feelsLikeC.toFixed(1) + "°C"]);
        }
        if (d.humidityPercent !== undefined) {
            out.push([i18ndc(domain, "Relative humidity", "Humidity"), Math.round(d.humidityPercent) + "%"]);
        }
        if (client.cloudCoverPercent >= 0) {
            out.push([i18ndc(domain, "Share of the sky covered by cloud", "Cloud cover"), client.cloudCoverPercent + "%"]);
        }
        if (d.windSpeedMps !== undefined) {
            out.push([i18ndc(domain, "Wind speed", "Wind"), d.windSpeedMps.toFixed(1) + " m/s"]);
        }
        if (d.airQualityIndex !== undefined) {
            out.push([i18ndc(domain, "US EPA air quality index", "Air quality"), String(Math.round(d.airQualityIndex))]);
        }
        return out;
    }

    /*! Label column: the longest reported label is 11 cells, so one cell of air. */
    readonly property int labelCells: 12
    /*! Two readings side by side when a label and a short value fit in each half. */
    readonly property bool twoColumns: frame.innerCols >= 2 * (labelCells + 6)
    readonly property int readingRows: twoColumns ? Math.ceil(readings.length / 2) : readings.length

    implicitHeight: frame.implicitHeight

    TermFrame {
        id: frame

        tm: term
        theme: root.theme
        width: root.width
        regionName: "weather"
        title: i18nd(root.domain, "Current conditions")
        rightRuns: {
            const t = root.theme;
            if (!root.client.hasWeatherReading) {
                return [{ t: "○ ", c: t.textMuted }, { t: i18ndc(root.domain, "Weather reading state", "No reading"), c: t.textMuted }];
            }
            if (root.client.weatherStale) {
                return [{ t: "▲ ", c: t.warning }, { t: i18ndc(root.domain, "Weather reading state", "Out of date"), c: t.warning }];
            }
            return [{ t: "● ", c: t.success }, { t: i18ndc(root.domain, "Weather reading state", "Up to date"), c: t.textMuted }];
        }
        contentRows: 2 + root.readingRows

        TermRuns {
            tm: term
            cells: frame.innerCols
            runs: {
                const condition = weatherText.conditionLabel(root.client.weatherCondition);
                const out = [];
                if (condition.length > 0) {
                    out.push({ t: condition, c: root.theme.fg, b: true });
                }
                if (root.client.hasTemperature) {
                    if (out.length > 0) {
                        out.push({ t: " · ", c: root.theme.borderInactive });
                    }
                    out.push({ t: root.client.temperatureC.toFixed(1) + "°C", c: root.theme.fg });
                }
                if (out.length === 0) {
                    out.push({ t: i18nd(root.domain, "No weather reading"), c: root.theme.textMuted });
                }
                return out;
            }
        }

        TermText {
            tm: term
            y: term.ch
            value: Term.padEnd(i18ndc(root.domain, "What the weather does to the brightness targets", "Effect"), root.labelCells)
            color: root.theme.textMuted
        }
        TermText {
            tm: term
            x: term.cx(root.labelCells)
            y: term.ch
            value: root.effect
            cells: Math.max(0, frame.innerCols - root.labelCells)
            color: root.client.hasWeatherReading ? root.theme.fg : root.theme.textMuted
        }

        Repeater {
            model: root.readings

            delegate: TermRuns {
                required property var modelData
                required property int index

                readonly property int half: Math.floor(frame.innerCols / 2)

                tm: term
                x: root.twoColumns && index % 2 === 1 ? term.cx(half) : 0
                y: term.cy(2 + (root.twoColumns ? Math.floor(index / 2) : index))
                cells: !root.twoColumns ? frame.innerCols
                    : (index % 2 === 1 ? frame.innerCols - half : half - 1)
                runs: [{ t: Term.padEnd(modelData[0], root.labelCells), c: root.theme.textMuted },
                       { t: modelData[1], c: root.theme.fg }]
            }
        }
    }
}
