/*
 * Forecast, PlayStation 3: the next intervals the daemon already fetched, as a
 * horizontal XMB strip. Each item is its time, a line glyph and the temperature,
 * with a hairline between items. Shown only when weather is enabled.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../components/weathericons"
import "../../modules"
import "../../Format.js" as Format

RowLayout {
    id: root

    // The PS3 side margin: the module's content keeps clear of the popup's edges.
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.leftMargin: root.style.sideMargin
    anchors.rightMargin: root.style.sideMargin

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property XmbStyle style: XmbStyle {
        theme: root.theme
    }

    /*! How many intervals to show; five fits the popup at its narrowest. */
    readonly property int slots: 5
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

    Layout.fillWidth: true
    spacing: 0

    Repeater {
        model: root.points

        delegate: Item {
            id: slot

            required property var modelData
            required property int index

            Layout.fillWidth: true
            implicitHeight: column.implicitHeight

            // The hairline between two items.
            Rectangle {
                visible: slot.index > 0
                anchors.left: parent.left
                anchors.top: parent.top
                anchors.bottom: parent.bottom
                anchors.topMargin: root.style.gap
                anchors.bottomMargin: root.style.gap
                width: root.style.hairline
                color: root.style.rule
            }

            ColumnLayout {
                id: column

                anchors.left: parent.left
                anchors.right: parent.right
                spacing: 0

                Text {
                    Layout.fillWidth: true
                    horizontalAlignment: Text.AlignHCenter
                    text: Format.timeText(slot.modelData.epochS)
                    textFormat: Text.PlainText
                    font: root.style.legendFont
                    color: root.style.inkSoft
                    elide: Text.ElideRight
                    lineHeightMode: Text.FixedHeight
                    lineHeight: root.style.sizeLegend
                }

                Row {
                    Layout.alignment: Qt.AlignHCenter
                    spacing: root.style.gap

                    ThemedWeatherIcon {
                        theme: root.theme
                        size: root.style.snap(root.style.unit * 0.9)
                        condition: slot.modelData.condition
                        night: slot.modelData.isNight === true
                        anchors.verticalCenter: parent.verticalCenter
                        WeatherGlyph {
                            anchors.centerIn: parent
                            size: root.style.snap(root.style.unit * 0.9)
                            condition: slot.modelData.condition
                            night: slot.modelData.isNight === true
                            glyphStyle: "line"
                            palette: root.theme.glyphPalette
                        }
                    }

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: i18ndc(root.domain, "Temperature in degrees, compact", "%1°",
                                     Math.round(slot.modelData.temperatureC))
                        textFormat: Text.PlainText
                        font: root.style.valueFont
                        color: root.style.ink
                        lineHeightMode: Text.FixedHeight
                        lineHeight: root.style.sizeBody
                    }
                }
            }
        }
    }
}
