/*
 * Forecast, macos: the next hours the daemon already fetched for its own policy,
 * as one rounded row. Under each hour, the sky in colour beside its temperature.
 * The number of hours follows the width.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }

    /*! Hours that fit: 3 to 5, one column per 72 logical px. */
    readonly property int slots: Math.max(3, Math.min(5, Math.floor(width / root.style.snap(72))))
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

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(root.domain, "Accessible name of the hourly forecast", "Forecast")

    Surface {
        Layout.fillWidth: true
        style: root.style
        padding: root.style.snap(4)
        radius: root.style.radiusCard

        RowLayout {
            Layout.fillWidth: true
            spacing: 0

            Repeater {
                model: root.points

                delegate: Item {
                    id: hour

                    required property var modelData

                    Layout.fillWidth: true
                    implicitWidth: column.implicitWidth
                    implicitHeight: column.implicitHeight

                    ColumnLayout {
                        id: column

                        anchors.fill: parent
                        spacing: root.style.snap(3)

                        Text {
                            Layout.alignment: Qt.AlignHCenter
                            text: Format.timeText(hour.modelData.epochS)
                            textFormat: Text.PlainText
                            font: root.style.caption
                            color: root.style.inkSecondary
                        }

                        // The sky and the temperature side by side under the hour.
                        RowLayout {
                            Layout.alignment: Qt.AlignHCenter
                            spacing: root.style.snap(4)

                            ThemedWeatherIcon {
                                theme: root.theme
                                size: root.style.snap(20)
                                condition: hour.modelData.condition || ""
                                night: hour.modelData.isNight === true
                                Layout.alignment: Qt.AlignVCenter
                                WeatherGlyph {
                                    anchors.centerIn: parent
                                    size: root.style.snap(20)
                                    condition: hour.modelData.condition || ""
                                    night: hour.modelData.isNight === true
                                    glyphStyle: "solid"
                                    palette: root.theme.glyphPalette
                                }
                            }

                            Text {
                                Layout.alignment: Qt.AlignVCenter
                                text: typeof hour.modelData.temperatureC === "number"
                                    ? i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                             Math.round(hour.modelData.temperatureC))
                                    : "—"
                                textFormat: Text.PlainText
                                font: root.style.bodyMedium
                                color: root.style.ink
                            }
                        }
                    }
                }
            }
        }
    }
}
