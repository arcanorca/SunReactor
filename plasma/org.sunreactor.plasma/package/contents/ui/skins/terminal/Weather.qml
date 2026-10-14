/*
 * Terminal skin of the weather module: the TUI's "Current conditions" panel
 * (src/tui/ui/weather.rs) as a picture and a reading.
 *
 *     ╭ Current conditions ───────────── ● Up to date ╮
 *     │  ▗▄▄▄▖         Partly cloudy                  │   sky picture, 13 x 7 cells,
 *     │ ▗█████▙▖       ██  ███  ███ ▄▄  ███           │   the temperature in a 5-row
 *     │▐█████████▌     ██  ███   ██ ▀▀  ███           │   block numeral font
 *     │ ▝▀▀▀▀▀▀▀▀▘     ███  ███  ██      ███           │
 *     │                ███  ███  ██        ███        │
 *     │                ███ ▄▄ ███ ▄▄  ███  ███        │   (rows 1-5)
 *     │                Dimming displays by 12%        │
 *     │ Feels 14° │ RH 62% │ Wind 3.4 m/s │ AQI 42     │
 *     ╰───────────────────────────────────────────────╯
 *
 * The daemon's bounded multiplier is said in words. Only readings the daemon
 * sent are listed; nothing is invented. The picture follows the condition and
 * the sun (night when the sun is down).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "Term.js" as Term
import "TermArt.js" as Art

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

    /*! The sun is down: the solar elevation when the daemon sent it, else the sun times. */
    readonly property bool night: {
        if (client.hasSolarElevation) {
            return client.solarElevation < 0;
        }
        const now = client.nowEpochS;
        return client.sunriseEpochS > 0 && client.sunsetEpochS > 0
            && (now < client.sunriseEpochS || now > client.sunsetEpochS);
    }

    /*! [label, value] pairs the daemon actually reported, short labels. */
    readonly property var readings: {
        const d = client.weatherDetails;
        const out = [];
        if (d.feelsLikeC !== undefined) {
            out.push([i18ndc(domain, "Short label: apparent temperature", "Feels"), Math.round(d.feelsLikeC) + "°"]);
        }
        if (d.humidityPercent !== undefined) {
            out.push([i18ndc(domain, "Short label: relative humidity", "RH"), Math.round(d.humidityPercent) + "%"]);
        }
        if (d.windSpeedMps !== undefined) {
            out.push([i18ndc(domain, "Wind speed", "Wind"), d.windSpeedMps.toFixed(1) + "m/s"]);
        }
        if (d.airQualityIndex !== undefined) {
            out.push([i18ndc(domain, "US EPA air quality index", "AQI"), String(Math.round(d.airQualityIndex))]);
        }
        return out;
    }

    /*! Cells the readings take, labels and values with one space between. */
    readonly property int detailCells: readings.reduce((n, pair) => n + Term.len(pair[0]) + 1 + Term.len(pair[1]), 0)
    /*! Spaces either side of the │ separators: one when the row fits, else none. */
    readonly property int detailPad: readings.length > 1 && detailCells + 3 * (readings.length - 1) <= frame.innerCols ? 1 : 0

    /*! The temperature as the numerals say it, e.g. "15.2°C". */
    readonly property string temperatureText: client.hasTemperature ? client.temperatureC.toFixed(1) + "°C" : ""
    readonly property var numerals: Art.bigText(temperatureText)
    readonly property var sky: Art.weatherArt(client.hasWeatherReading ? client.weatherCondition : "", night)
    readonly property int artCols: Math.max(...sky.rows.map(row => Array.from(row).length))
    /*! Columns right of the picture: the numerals and the effect. */
    readonly property int textCols: Math.max(0, frame.innerCols - artCols - 2)
    readonly property bool bigNumerals: client.hasWeatherReading && client.hasTemperature
        && numerals.rows[0].length <= textCols

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
        // Seven picture rows, then the details row.
        contentRows: 8

        // The sky picture.
        TermPicture {
            tm: term
            theme: root.theme
            x: 0
            y: term.cy(0)
            rows: root.sky.rows
            roles: root.sky.roles
            colours: ({ o: root.theme.accent, h: root.theme.secondaryAccent, c: root.theme.mix(root.theme.fg, root.theme.bg, 0.15),
                        m: root.theme.textMuted, y: root.theme.warning, w: root.theme.fg,
                        d: root.theme.borderInactive })
        }

        // The condition, on the right of the picture's top row.
        TermText {
            tm: term
            x: term.cx(root.artCols + 2)
            y: term.cy(0)
            cells: root.textCols
            value: root.client.hasWeatherReading ? weatherText.conditionLabel(root.client.weatherCondition)
                : i18nd(root.domain, "No weather reading")
            strong: true
            color: root.client.hasWeatherReading ? root.theme.fg : root.theme.textMuted
        }

        // The temperature, in block numerals (rows 1-5).
        TermPicture {
            visible: root.bigNumerals
            tm: term
            theme: root.theme
            x: term.cx(root.artCols + 2)
            y: term.cy(1)
            rows: root.numerals.rows
            roles: root.numerals.roles
            colours: ({ o: root.theme.fg })
        }
        // No numerals (no temperature yet): the status says why, in text.
        TermText {
            visible: !root.bigNumerals
            tm: term
            x: term.cx(root.artCols + 2)
            y: term.cy(1)
            cells: root.textCols
            value: root.client.hasTemperature ? root.temperatureText : root.effect
            strong: root.client.hasTemperature
            color: root.theme.textMuted
        }

        // What the weather does to the brightness (row 6, right of the picture).
        TermText {
            tm: term
            visible: root.client.hasWeatherReading
            x: term.cx(root.artCols + 2)
            y: term.cy(6)
            cells: root.textCols
            value: root.effect
            color: root.theme.fg
        }

        // The details: key value pairs separated by │, one row, cut to fit.
        Rectangle {
            x: 0
            y: term.cy(7)
            width: term.cx(frame.innerCols)
            height: term.lw
            color: root.theme.borderInactive
            antialiasing: false
        }
        TermRuns {
            id: details

            tm: term
            y: term.cy(7)
            cells: frame.innerCols
            defaultColor: root.theme.textMuted
            runs: {
                const t = root.theme;
                const pad = " ".repeat(root.detailPad);
                const out = [];
                root.readings.forEach((pair, i) => {
                    if (i > 0) {
                        out.push({ t: pad + "│" + pad, c: t.borderInactive });
                    }
                    out.push({ t: pair[0] + " ", c: t.textMuted });
                    out.push({ t: pair[1], c: t.fg, b: true });
                });
                return out;
            }
        }
    }
}
