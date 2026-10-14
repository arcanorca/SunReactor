/*
 * Terminal skin of the sun times module: the day as one track, from sunrise to
 * sunset, with the time now as ●. Two rows, no box: it sits under the forecast.
 *
 *     ↑ 7:13 AM  ━━━━━━━━━━━━●━━━━━━━━━━━━━━━━━  6:38 PM ↓
 *     Daylight 11h 25m                 Solar noon 12:55 PM
 *
 * The part of the day that has passed is in the accent colour; the dot is
 * clamped to the track's ends at night. With the solar elevation enabled, the
 * second row shows the sun's angle instead of the noon.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../Format.js" as Format
import "Term.js" as Term

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.sunriseEpochS > 0 && client.sunsetEpochS > 0
    readonly property bool showElevation: settings.showSolarElevation && client.hasSolarElevation

    TermMetrics {
        id: term
        theme: root.theme
    }

    /*! Cells inside the same margins as a framed module's body. */
    readonly property int innerCols: Math.max(0, term.cols(root.width) - 4)
    readonly property real originX: term.inset(root.width) + term.cx(2)
    /*! Cells of an end label ("↑ 7:13 AM"), and of the gap between label and track. */
    readonly property int labelCells: 10
    readonly property int trackStart: labelCells + 1
    readonly property int trackCells: Math.max(2, innerCols - 2 * (labelCells + 1))
    /*! Where the dot is along the day: 0 at sunrise, 1 at sunset (clamped). */
    readonly property real progress: {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (!(set > rise)) {
            return 0;
        }
        return Math.max(0, Math.min(1, (client.nowEpochS - rise) / (set - rise)));
    }
    readonly property int dotCell: Math.round(progress * (trackCells - 1))

    implicitHeight: term.cy(2)

    // Row 1: the sunrise, the track and the sunset.
    TermRuns {
        id: sunriseLabel

        tm: term
        visible: root.available
        x: root.originX
        runs: [{ t: "↑ ", c: root.theme.accent, b: true },
               { t: Format.timeText(root.client.sunriseEpochS), c: root.theme.fg, b: true }]
    }
    TermRuns {
        id: sunsetLabel

        tm: term
        visible: root.available
        x: root.originX + term.cx(root.innerCols - sunsetLabel.usedCells)
        runs: [{ t: Format.timeText(root.client.sunsetEpochS), c: root.theme.fg, b: true },
               { t: " ↓", c: root.theme.accent, b: true }]
    }

    // The track: a heavy line on the row's centre, accent up to the dot.
    Rectangle {
        visible: root.available
        x: root.originX + term.cx(root.trackStart)
        y: term.lineY(0) - Math.floor(term.lwDev / 2) / term.dpr
        width: term.cx(root.trackCells)
        height: term.lwHeavy
        color: root.theme.borderInactive
        antialiasing: false
    }
    Rectangle {
        visible: root.available && root.dotCell > 0
        x: root.originX + term.cx(root.trackStart)
        y: term.lineY(0) - Math.floor(term.lwDev / 2) / term.dpr
        width: term.cx(root.dotCell)
        height: term.lwHeavy
        color: root.theme.accent
        antialiasing: false
    }
    TermText {
        tm: term
        visible: root.available
        x: root.originX + term.cx(root.trackStart + root.dotCell)
        value: "●"
        strong: true
        color: root.theme.fg
    }

    // Without sun times yet, the row says so.
    TermText {
        tm: term
        visible: !root.available
        x: root.originX
        cells: root.innerCols
        value: i18nd(root.domain, "Sun times are not known yet")
        color: root.theme.textMuted
    }

    // Row 2: the day's length, and the noon or the sun's angle.
    TermRuns {
        tm: term
        y: term.cy(1)
        x: root.originX
        cells: root.innerCols
        visible: root.available
        runs: [{ t: i18nd(root.domain, "Daylight") + " ", c: root.theme.textMuted },
               { t: Term.duration(root.client.sunsetEpochS - root.client.sunriseEpochS), c: root.theme.fg, b: true }]
    }
    TermRuns {
        tm: term
        y: term.cy(1)
        visible: root.available
        x: root.originX + term.cx(root.innerCols - usedCells)
        runs: root.showElevation
            ? [{ t: i18nd(root.domain, "Sun now") + " ", c: root.theme.textMuted },
               { t: root.client.solarElevation.toFixed(1) + "°", c: root.theme.fg, b: true }]
            : [{ t: i18nd(root.domain, "Solar noon") + " ", c: root.theme.textMuted },
               { t: root.client.solarNoonEpochS > 0 ? Format.timeText(root.client.solarNoonEpochS) : "—", c: root.theme.fg, b: true }]
    }
}
