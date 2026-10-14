/*
 * hackergreen forecast: the next intervals the daemon fetched for its own policy,
 * one ASCII sky per slot, the temperature under it.
 *
 *   == FORECAST ===========================================  next 5 h
 *     15:00     16:00     17:00     18:00     19:00
 *      .-.       .-.     ' ' '
 *     (   )     (   )     ...
 *     (___)     (___)
 *      15°       14°       13°
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../Format.js" as Format
import "../terminal"
import "Hg.js" as Hg

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain

    TermMetrics {
        id: term

        theme: root.theme
    }
    HgCrt {
        theme: root.theme
    }


    /*! Slots that fit: each wants nine cells, at most five. */
    readonly property int slots: Math.max(1, Math.min(5, Math.floor(term.cols(root.width) / 9)))
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === slots) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    implicitHeight: section.implicitHeight

    function isNight(epochS) {
        return client.sunriseEpochS > 0 && client.sunsetEpochS > 0
            && (epochS < client.sunriseEpochS || epochS > client.sunsetEpochS);
    }

    HgSection {
        id: section

        tm: term
        theme: root.theme
        width: root.width
        title: i18ndc(root.domain, "Section title of the forecast", "FORECAST")
        metaRuns: root.points.length > 0
            ? [{ t: i18ndp(root.domain, "next hour", "next %1 h",
                           Math.max(1, Math.round((root.points[root.points.length - 1].epochS - root.client.nowEpochS) / 3600))),
                 c: root.theme.textMuted }]
            : []
        bodyHeight: term.cy(5)
        glowing: true

        Repeater {
            model: root.points

            delegate: Item {
                id: slot

                required property var modelData
                required property int index

                readonly property int w: Math.floor(term.cols(root.width) / root.slots)
                readonly property var rows: Hg.art(modelData.condition, root.isNight(modelData.epochS))
                x: term.cx(index * w)
                width: term.cx(w)
                height: term.cy(5)

                function centred(cells) {
                    return term.cx(Math.max(0, Math.floor((w - cells) / 2)));
                }

                TermText {
                    tm: term
                    x: slot.centred(usedCells)
                    value: Format.timeText(slot.modelData.epochS)
                    cells: slot.w - 1
                    color: root.theme.textMuted
                }

                // The three art rows, or the chosen Weather icons style in their place.
                ThemedWeatherIcon {
                    x: slot.centred(7)
                    y: term.cy(1)
                    theme: root.theme
                    condition: slot.modelData.condition
                    night: root.isNight(slot.modelData.epochS)
                    size: Math.min(term.cy(4) - term.cy(1), term.cx(7))

                    Repeater {
                        model: slot.rows

                        delegate: TermText {
                            required property string modelData
                            required property int index

                            tm: term
                            x: 0
                            y: term.cy(1 + index) - term.cy(1)
                            value: modelData
                            color: root.theme.fg
                        }
                    }
                }

                TermText {
                    tm: term
                    y: term.cy(4)
                    x: slot.centred(usedCells)
                    value: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                  Math.round(slot.modelData.temperatureC))
                    strong: true
                    color: root.theme.accent
                }

            }
        }
    }
}
