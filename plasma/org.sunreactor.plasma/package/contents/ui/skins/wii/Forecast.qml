/*
 * Forecast, Wii style: the next intervals the daemon already fetched, as a row
 * of small channel tiles: the time, the weather glyph, the temperature. Like
 * the Forecast Channel's day strip; it reads the same data as the instrument's.
 *
 *   [ 14:00 ]  [ 15:00 ]  [ 16:00 ]  [ 17:00 ]  [ 18:00 ]
 *   [  (sun) ] [  (cloud)] ...
 *   [   15°  ] [   14°   ] ...
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components/weathericons"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property WiiStyle style: WiiStyle {
        theme: root.theme
    }

    /*! The first five intervals after now. */
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === 5) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && points.length > 0

    /*! Whether an instant falls outside the daylight hours of the sun times. */
    function isNight(epochS) {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (!(rise > 0) || !(set > 0)) {
            return false;
        }
        return epochS < rise || epochS > set;
    }

    spacing: style.rowGap

    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap

        Repeater {
            model: root.points

            delegate: WiiTile {
                id: slot

                required property var modelData

                style: root.style
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                pad: root.style.gap
                implicitHeight: column.implicitHeight + 2 * slot.pad

                ColumnLayout {
                    id: column

                    anchors.fill: parent
                    spacing: 0

                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: Format.timeText(slot.modelData.epochS)
                        textFormat: Text.PlainText
                        font: root.style.captionFont
                        color: root.style.inkSoft
                    }
                    WiiWeatherIcon {
                        Layout.alignment: Qt.AlignHCenter
                        theme: root.theme
                        size: root.style.snap(root.style.unit * 1.1)
                        condition: slot.modelData.condition
                        night: root.isNight(slot.modelData.epochS)
                    }
                    Text {
                        Layout.alignment: Qt.AlignHCenter
                        text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                     Math.round(slot.modelData.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.bodyBoldFont
                        color: root.style.ink
                    }
                }
            }
        }
    }
}
