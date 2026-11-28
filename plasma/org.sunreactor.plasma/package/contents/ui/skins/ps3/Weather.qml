/*
 * Weather, PlayStation 3: the reading as a large light temperature beside its
 * line glyph, the condition under it, and what the reading does to the targets
 * in sub-text. The secondary readings are a vertical XMB list with thin
 * separators. Shown only when weather is enabled.
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

ColumnLayout {
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
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    readonly property string stateNote: client.hasWeatherReading ? "" : weatherText.stateNote(client.weatherState)

    readonly property string effectText: client.hasWeatherReading ? weatherText.effect(client) : ""

    /*! The secondary readings, in the order of the list. */
    readonly property var details: weatherText.detailsListInUnit(client.weatherDetails, "full", client.isFahrenheit)

    WeatherText {
        id: weatherText
    }

    Layout.fillWidth: true
    spacing: style.gap * 0.5

    // The reading: the glyph, the temperature with the condition under it, and
    // what the reading does to the targets, in sub-text on the right.
    RowLayout {
        Layout.fillWidth: true
        spacing: root.style.gap2

        ThemedWeatherIcon {
            theme: root.theme
            size: root.style.snap(root.style.sizeHero * 1.1)
            condition: root.client.weatherCondition
            night: root.client.weatherIsNight
            Layout.alignment: Qt.AlignTop
            visible: root.client.hasWeatherReading
            Ps3Icon {
                anchors.centerIn: parent
                theme: root.theme
                size: root.style.snap(root.style.sizeHero * 1.1)
                condition: root.client.weatherCondition
                night: root.client.weatherIsNight
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.gap

                RowLayout {
                    visible: root.client.hasTemperature
                    spacing: root.style.gap * 0.5

                    Text {
                        text: String(Format.tempRounded(root.client.temperatureC, root.client.isFahrenheit))
                        textFormat: Text.PlainText
                        font: root.style.heroFont
                        color: root.style.ink
                        style: Text.Raised
                        styleColor: root.style.textShadow
                        lineHeightMode: Text.FixedHeight
                        lineHeight: root.style.sizeHero
                    }

                    Text {
                        Layout.alignment: Qt.AlignBottom
                        Layout.bottomMargin: root.style.snap(root.style.sizeHero * 0.12)
                        text: Format.tempUnit(root.client.isFahrenheit)
                        textFormat: Text.PlainText
                        font: root.style.unitFont
                        color: root.style.inkSoft
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                Text {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.maximumWidth: root.width * 0.5
                    visible: text.length > 0
                    text: root.effectText
                    textFormat: Text.PlainText
                    horizontalAlignment: Text.AlignRight
                    font: root.style.legendFont
                    color: root.style.inkSoft
                    elide: Text.ElideLeft
                }
            }

            Text {
                Layout.fillWidth: true
                text: root.client.hasWeatherReading
                    ? (weatherText.conditionLabel(root.client.weatherCondition)
                       || i18nd(root.domain, "No weather reading"))
                    : root.stateNote
                textFormat: Text.PlainText
                font: root.style.labelFont
                color: root.client.hasWeatherReading ? root.style.ink : root.style.inkSoft
                style: Text.Raised
                styleColor: root.style.textShadow
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                visible: root.client.locationCity.length > 0
                text: root.client.locationCity
                textFormat: Text.PlainText
                font: root.style.legendFont
                color: root.style.inkSoft
                style: Text.Raised
                styleColor: root.style.textShadow
                elide: Text.ElideRight
            }
        }
    }

    // The secondary readings: a vertical XMB list.
    ColumnLayout {
        Layout.fillWidth: true
        visible: root.client.hasWeatherReading && root.details.length > 0
        spacing: 0

        Repeater {
            model: root.details

            delegate: XmbRow {
                required property var modelData

                style: root.style
                padding: root.style.snap(root.style.gap * 0.25)
                label: modelData.k
                value: modelData.v
            }
        }
    }
}
