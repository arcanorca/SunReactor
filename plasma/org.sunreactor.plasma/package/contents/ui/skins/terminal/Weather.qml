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
import "../../components/weathericons"
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

    /*! The phrases of the status, longest first: the one that fits the text
        column is used (the labels are fixed, so they are shortened, never cut). */
    readonly property var effectForms: {
        if (!client.hasWeatherReading) {
            switch (client.weatherState) {
            case "no_api_key":
                return [i18nd(domain, "No API key is configured"), i18nd(domain, "No API key")];
            case "loading":
                return [i18nd(domain, "Looking up the current conditions…"), i18nd(domain, "Looking up conditions…"),
                        i18nd(domain, "Looking up…")];
            case "unauthorized":
                return [i18nd(domain, "The weather provider rejected the API key"), i18nd(domain, "API key rejected")];
            case "rate_limited":
                return [i18nd(domain, "The weather provider is rate limiting requests"), i18nd(domain, "Rate limited")];
            case "network_error":
                return [i18nd(domain, "The weather provider could not be reached"), i18nd(domain, "Provider unreachable")];
            default:
                return [i18nd(domain, "No reading yet")];
            }
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return [i18ndc(domain, "Placeholder is a percentage", "Dimming displays by %1%", change),
                    i18ndc(domain, "Placeholder is a percentage", "Dimming by %1%", change),
                    i18ndc(domain, "Placeholder is a percentage", "Dim %1%", change)];
        }
        if (change <= -2) {
            return [i18ndc(domain, "Placeholder is a percentage", "Brightening displays by %1%", -change),
                    i18ndc(domain, "Placeholder is a percentage", "Brightening by %1%", -change),
                    i18ndc(domain, "Placeholder is a percentage", "Brighter %1%", -change)];
        }
        return [i18nd(domain, "Not changing brightness"), i18nd(domain, "No change")];
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

    /*! Cells of a row of readings: label, space, value each, and a separator of
        `pad` spaces either side of the │ between them. */
    function detailCellsOf(items, pad) {
        const cells = items.reduce((n, pair) => n + Term.len(pair[0]) + 1 + Term.len(pair[1]), 0);
        return cells + Math.max(0, items.length - 1) * (1 + 2 * pad);
    }
    /*! The readings shown: all of them when they fit the row, else the first ones
        that do (the last reported reading goes first, never a cut value). */
    readonly property var shownReadings: {
        const list = root.readings.slice();
        while (list.length > 1 && root.detailCellsOf(list, 0) > frame.innerCols) {
            list.pop();
        }
        return list;
    }
    /*! Spaces either side of the │ separators: one when the row fits, else none. */
    readonly property int detailPad: shownReadings.length > 1
        && detailCellsOf(shownReadings, 1) <= frame.innerCols ? 1 : 0

    /*! The status phrase in the longest form that fits the text column. */
    readonly property string effect: Term.firstFit(effectForms, textCols)

    /*! The temperature as the numerals say it, e.g. "15.2°C". */
    readonly property string temperatureText: client.hasTemperature ? client.temperatureC.toFixed(1) + "°C" : ""
    readonly property var numerals: Art.bigText(temperatureText)
    readonly property var numeralsSmall: Art.smallText(temperatureText)
    readonly property var sky: Art.weatherArt(client.hasWeatherReading ? client.weatherCondition : "", night)
    readonly property int artCols: Math.max(...sky.rows.map(row => Array.from(row).length))
    /*! Columns right of the picture: the numerals and the effect. */
    readonly property int textCols: Math.max(0, frame.innerCols - artCols - 2)
    /*! The temperature drawn in the largest numerals that fit the text column:
        "big" (5 rows), "small" (3 rows), or "" for the words. */
    readonly property string numeralSet: !(client.hasWeatherReading && client.hasTemperature) ? ""
        : numerals.rows[0].length <= textCols ? "big"
        : numeralsSmall.rows[0].length <= textCols ? "small" : ""
    readonly property bool bigNumerals: numeralSet !== ""

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

        // The sky picture, or the chosen Weather icons style in its place: the largest
        // square that fits the seven picture rows and the picture's columns.
        ThemedWeatherIcon {
            x: 0
            y: term.cy(0)
            theme: root.theme
            condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
            night: root.night
            size: Math.min(term.cy(7) - term.cy(0), term.cx(root.artCols))

            TermPicture {
                tm: term
                theme: root.theme
                x: 0
                y: 0
                rows: root.sky.rows
                roles: root.sky.roles
                colours: ({ o: root.theme.accent, h: root.theme.secondaryAccent, c: root.theme.mix(root.theme.fg, root.theme.bg, 0.15),
                            m: root.theme.textMuted, y: root.theme.warning, w: root.theme.fg,
                            d: root.theme.borderInactive })
            }
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

        // The temperature, in block numerals (rows 1-5, or 1-3 in the small set).
        TermPicture {
            visible: root.numeralSet !== ""
            tm: term
            theme: root.theme
            x: term.cx(root.artCols + 2)
            y: term.cy(1)
            rows: root.numeralSet === "small" ? root.numeralsSmall.rows : root.numerals.rows
            roles: root.numeralSet === "small" ? root.numeralsSmall.roles : root.numerals.roles
            colours: ({ o: root.theme.fg })
        }
        // No numerals (no temperature yet, or no room): the words, in the longest form that fits.
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
                root.shownReadings.forEach((pair, i) => {
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
