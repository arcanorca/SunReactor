/*
 * Nintendo DS touch screen, forecast: the next intervals the daemon already
 * fetched as white rounded tiles - time, DS cartoon sky icon, temperature - up to
 * five across, then a details list: what the weather does to the displays and
 * whether the reading is live. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
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
    readonly property int maxSlots: 5
    readonly property var points: {
        const upcoming = [];
        for (const point of client.forecast) {
            if (point.epochS > client.nowEpochS) {
                upcoming.push(point);
            }
            if (upcoming.length === maxSlots) {
                break;
            }
        }
        return upcoming;
    }
    readonly property bool available: client.isConnected && client.weatherEnabled && client.forecast.length > 0

    /*! What the daemon's bounded weather multiplier does to the targets, in words. */
    readonly property string effect: {
        if (!client.hasWeatherReading) {
            return client.weatherState === "no_api_key"
                ? i18nd(domain, "No API key is configured")
                : i18nd(domain, "No reading yet");
        }
        if (client.weatherStale) {
            return i18nd(domain, "Last reading is out of date");
        }
        const change = Math.round((1.0 - client.weatherMultiplier) * 100);
        if (change >= 2) {
            return i18ndc(domain, "Placeholder is a percentage", "Dimming displays by %1%", change);
        }
        if (change <= -2) {
            return i18ndc(domain, "Placeholder is a percentage", "Brightening displays by %1%", -change);
        }
        return i18nd(domain, "Not changing brightness right now");
    }
    readonly property var details: [
        { label: i18nd(domain, "Brightness"), value: effect },
        { label: i18nd(domain, "Reading"), value: reading }
    ]
    readonly property string reading: {
        if (!client.hasWeatherReading) {
            return client.weatherState === "no_api_key" ? i18nd(domain, "No key") : i18nd(domain, "None yet");
        }
        return client.weatherStale ? i18nd(domain, "Out of date") : i18nd(domain, "Live");
    }

    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.35)
    readonly property real inset: Math.round(gu * 0.6)
    readonly property real tileGap: Math.round(gu * 0.25)
    readonly property real tileH: Math.round(gu * 3.3)
    readonly property real detailsTop: pad + tileH + Math.round(gu * 0.4)
    readonly property real detailRowH: Math.round(gu * 1.0)
    readonly property real detailsH: details.length * detailRowH + Math.round(gu * 0.4)
    readonly property real glassH: detailsTop + detailsH + pad

    implicitHeight: screen.implicitHeight

    Accessible.role: Accessible.List
    Accessible.name: i18ndc(domain, "Title of the forecast panel", "Forecast")

    NdsScreen {
        id: screen

        width: parent.width
        theme: root.theme
        style: root.style
        wallpaper: "touch"
        glassHeight: root.glassH

        Repeater {
            model: root.points

            delegate: ForecastTile {
            }
        }

        Rectangle {
            id: detailsCard

            x: root.inset
            y: root.detailsTop
            width: screen.glassWidth - 2 * root.inset
            height: root.detailsH
            radius: Math.round(root.style.radius * 0.85)
            color: root.style.white
            border.width: 1
            border.color: root.style.line
        }

        Repeater {
            model: root.details

            delegate: Item {
                id: detailRow

                required property var modelData
                required property int index

                x: root.inset
                y: root.detailsTop + Math.round(root.gu * 0.2) + index * root.detailRowH
                width: detailsCard.width
                height: root.detailRowH

                Accessible.role: Accessible.ListItem
                Accessible.name: modelData.label + " " + modelData.value

                Text {
                    x: Math.round(root.gu * 0.8)
                    anchors.verticalCenter: parent.verticalCenter
                    text: detailRow.modelData.label
                    font.family: root.style.family
                    font.pixelSize: Math.round(root.gu * 0.72)
                    color: root.style.muted
                }
                Text {
                    anchors.right: parent.right
                    anchors.rightMargin: Math.round(root.gu * 0.8)
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.round(parent.width * 0.62)
                    horizontalAlignment: Text.AlignRight
                    elide: Text.ElideRight
                    text: detailRow.modelData.value
                    font.family: root.style.family
                    font.weight: Font.Bold
                    font.pixelSize: Math.round(root.gu * 0.8)
                    color: root.style.ink
                }
            }
        }
    }

    /*! One forecast interval: a white tile with its time, sky and temperature. */
    component ForecastTile: NdsPanel {
        id: tile

        required property var modelData
        required property int index

        readonly property int count: Math.max(1, root.points.length)

        style: root.style
        kind: "panel"
        outlineColor: root.style.line
        outlineWidth: 1
        radius: Math.round(root.style.radius * 0.85)
        width: Math.floor((screen.glassWidth - 2 * root.inset - (count - 1) * root.tileGap) / count)
        height: root.tileH
        x: root.inset + index * (width + root.tileGap)
        y: root.pad

        Accessible.role: Accessible.ListItem
        Accessible.name: Format.timeText(modelData.epochS) + " " + Math.round(modelData.temperatureC) + "°"

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.round(root.gu * 0.2)
            text: Format.timeText(tile.modelData.epochS)
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.6)
            color: root.style.muted
        }
        NdsWeatherIcon {
            anchors.horizontalCenter: parent.horizontalCenter
            y: Math.round(root.gu * 0.8)
            size: Math.round(Math.min(tile.width * 0.7, root.gu * 1.4))
            theme: root.theme
            condition: tile.modelData.condition || ""
            night: tile.modelData.isNight === true
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            y: tile.height - Math.round(root.gu * 1.1)
            text: i18ndc(root.settings.domain, "Temperature in degrees, compact", "%1°",
                         Math.round(tile.modelData.temperatureC))
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.85)
            color: root.style.ink
        }
    }
}
