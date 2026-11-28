/*
 * Nintendo DS top screen, weather page: the current sky as the DS cartoon icon
 * (components/weathericons/NdsWeatherIcon.qml) standing on the screen with no
 * plate, the temperature in big rounded digits, the condition and what the
 * daemon's weather does to the displays. The hinge sits under the screen.
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"
import "../../components/weathericons"
import "../../Format.js" as Format

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: (client.isConnected || client.statusStale) && client.weatherEnabled

    readonly property string condition: {
        const text = weatherText.conditionLabel(client.weatherCondition);
        return client.hasWeatherReading && text.length > 0 ? text : i18nd(domain, "No weather reading");
    }

    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.75)
    readonly property real glassW: root.width - 2 * style.bezel
    readonly property real topBarH: Math.round(gu * 1.4)
    readonly property real badge: Math.round(gu * 3.4)
    readonly property real glassH: Math.round(gu * (root.client.locationCity.length > 0 ? 6.9 : 6.0))

    WeatherText {
        id: weatherText
    }

    implicitHeight: screen.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: weatherText.summary(client)

    NdsScreen {
        id: screen

        width: parent.width
        theme: root.theme
        style: root.style
        wallpaper: "sky"
        hinge: true
        daemonMode: root.client.mode
        glassHeight: root.glassH

        Rectangle {
            x: 0
            y: 0
            width: root.glassW
            height: root.topBarH
            color: root.style.blue

            NdsStatusBar {
                x: root.pad
                anchors.verticalCenter: parent.verticalCenter
                width: root.glassW - 2 * root.pad
                theme: root.theme
                style: root.style
                client: root.client
                onAccentBar: true
            }

            Rectangle {
                x: 0
                y: parent.height - 1
                width: parent.width
                height: 1
                color: root.style.frameBlack
            }
        }

        // The sky: the DS cartoon icon in the slot the badge used to fill.
        Item {
            id: badgeBox

            x: root.pad
            y: root.topBarH + Math.round(root.gu * 0.55)
            width: root.badge
            height: root.badge
        }
        ThemedWeatherIcon {
            theme: root.theme
            size: Math.round(root.gu * 2.9)
            condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
            night: root.client.weatherIsNight === true
            x: badgeBox.x + Math.round((badgeBox.width - width) / 2)
            y: badgeBox.y + Math.round((badgeBox.height - height) / 2)
            NdsWeatherIcon {
                anchors.centerIn: parent
                size: Math.round(root.gu * 2.9)
                theme: root.theme
                condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
                night: root.client.weatherIsNight === true
            }
        }

        // The temperature: crisp digits, the unit beside them.
        Text {
            id: temperature

            x: badgeBox.x + root.badge + Math.round(root.gu * 0.8)
            y: root.topBarH + Math.round(root.gu * 0.35)
            text: root.client.hasTemperature ? String(Format.tempRounded(root.client.temperatureC, root.client.isFahrenheit)) : "--"
            textFormat: Text.PlainText
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 2.0)
            color: root.style.ink
        }
        Text {
            x: temperature.x + temperature.implicitWidth + Math.round(root.gu * 0.2)
            y: root.topBarH + Math.round(root.gu * 0.75)
            text: Format.tempUnit(root.client.isFahrenheit)
            textFormat: Text.PlainText
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.0)
            color: root.style.accentInk
        }

        Text {
            id: conditionText

            x: badgeBox.x + root.badge + Math.round(root.gu * 0.8)
            y: root.topBarH + Math.round(root.gu * 3.05)
            width: root.glassW - x - root.pad
            elide: Text.ElideRight
            text: root.condition
            textFormat: Text.PlainText
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.0)
            color: root.style.ink
        }

        Text {
            visible: root.client.locationCity.length > 0
            x: conditionText.x
            y: conditionText.y + conditionText.implicitHeight + Math.round(root.gu * 0.12)
            width: root.glassW - x - root.pad
            elide: Text.ElideRight
            text: root.client.locationCity
            textFormat: Text.PlainText
            font.family: root.style.family
            font.weight: Font.DemiBold
            font.pixelSize: Math.round(root.gu * 0.80)
            color: root.style.muted
        }
    }
}
