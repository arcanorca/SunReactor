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

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected && client.weatherEnabled

    readonly property string condition: {
        const text = weatherText.conditionLabel(client.weatherCondition);
        return client.hasWeatherReading && text.length > 0 ? text : i18nd(domain, "No weather reading");
    }

    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.75)
    readonly property real glassW: root.width - 2 * style.bezel
    readonly property real badge: Math.round(gu * 3.4)
    readonly property real glassH: Math.round(gu * 5.8)

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
        glassHeight: root.glassH

        NdsStatusBar {
            x: root.pad
            y: Math.round(root.gu * 0.35)
            width: root.glassW - 2 * root.pad
            theme: root.theme
            style: root.style
            client: root.client
        }

        // The sky: the DS cartoon icon in the slot the badge used to fill.
        Item {
            id: badgeBox

            x: root.pad
            y: Math.round(root.gu * 1.8)
            width: root.badge
            height: root.badge
        }
        NdsWeatherIcon {
            x: badgeBox.x + Math.round((badgeBox.width - width) / 2)
            y: badgeBox.y + Math.round((badgeBox.height - height) / 2)
            size: Math.round(root.gu * 2.9)
            theme: root.theme
            condition: root.client.hasWeatherReading ? root.client.weatherCondition : ""
            night: root.client.weatherIsNight === true
        }

        // The temperature: big rounded digits, the unit beside them.
        Text {
            id: temperature

            x: badgeBox.x + root.badge + Math.round(root.gu * 0.8)
            y: Math.round(root.gu * 1.6)
            text: root.client.hasTemperature ? String(Math.round(root.client.temperatureC)) : "--"
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 2.0)
            color: root.style.ink
        }
        Text {
            x: temperature.x + temperature.implicitWidth + Math.round(root.gu * 0.2)
            y: Math.round(root.gu * 2.0)
            text: "°C"
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.0)
            color: root.style.blue
        }

        Text {
            x: badgeBox.x + root.badge + Math.round(root.gu * 0.8)
            y: Math.round(root.gu * 4.35)
            width: root.glassW - x - root.pad
            elide: Text.ElideRight
            text: root.condition
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 1.0)
            color: root.style.ink
        }
    }
}
