/*
 * Temperature and (opt-in) sun height beside the panel graphic, in the
 * panel's text colour. Pixel themes use the bitmap font (crisp at any scale)
 * where it fits; a vertical panel narrower than the glyphs gets vector text.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "../Format.js" as Format

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    property bool showTemperature: true
    property bool showElevation: false
    property bool vertical: false
    /*! Width available (vertical panels); infinite otherwise. */
    property real available: Number.POSITIVE_INFINITY
    property color color: Kirigami.Theme.textColor

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property bool temperatureShown: showTemperature && (client.isConnected || client.statusStale) && client.hasTemperature
    readonly property bool elevationShown: showElevation && (client.isConnected || client.statusStale) && client.hasSolarElevation
    readonly property int visibleCount: (temperatureShown ? 1 : 0) + (elevationShown ? 1 : 0)

    readonly property string temperatureText: i18ndc(domain, "Temperature in degrees, panel-sized",
                                                      "%1°", Format.tempRounded(client.temperatureC, client.isFahrenheit))
    readonly property string elevationText: i18ndc(domain, "Sun height above the horizon, panel-sized",
                                                    "%1°", client.solarElevation.toFixed(1))
    readonly property bool bitmap: theme.isPixel
        && Math.max(bitmapTemperature.implicitWidth, elevationShown ? bitmapElevation.implicitWidth : 0) <= available
    /*! Nothing: compact 3x5 dot face at panel pitch so Nothing matches other themes' panel footprint. */
    readonly property bool dotted: theme.themeId === "nothing"
    property real dotPitch: 3 / theme.grid.devicePixelRatio
    readonly property real compactDotPitch: {
        const dpr = Math.max(1, theme.grid.devicePixelRatio);
        const capPx = Math.max(2, Math.round(theme.fontSizeBody * dpr / 5.5)) / dpr;
        return Math.min(dotPitch, capPx);
    }
    readonly property string dotFace: "tiny"

    spacing: 0

    BitmapText {
        id: bitmapTemperature
        Layout.alignment: Qt.AlignCenter
        visible: root.bitmap && root.temperatureShown
        theme: root.theme
        text: root.temperatureText
        color: root.color
    }

    BitmapText {
        id: bitmapElevation
        Layout.alignment: Qt.AlignCenter
        visible: root.bitmap && root.elevationShown
        theme: root.theme
        text: root.elevationText
        color: root.color
    }

    DotText {
        Layout.alignment: Qt.AlignCenter
        visible: root.dotted && root.temperatureShown
        text: root.temperatureText
        face: root.dotFace
        size: 1
        pitch: root.compactDotPitch
        litColor: root.color
        dimColor: "transparent"
    }

    DotText {
        Layout.alignment: Qt.AlignCenter
        visible: root.dotted && root.elevationShown
        text: root.elevationText
        face: root.dotFace
        size: 1
        pitch: root.compactDotPitch
        litColor: root.color
        dimColor: "transparent"
    }

    PlasmaComponents3.Label {
        Layout.alignment: Qt.AlignCenter
        visible: !root.bitmap && !root.dotted && root.temperatureShown
        text: root.temperatureText
        textFormat: Text.PlainText
        color: root.color
        font.features: ({ "tnum": 1 })
        font.family: root.theme.digitsFont
        font.pixelSize: root.vertical || root.elevationShown
            ? root.theme.fontSizeSmall
            : root.theme.fontSizeBody
    }

    PlasmaComponents3.Label {
        Layout.alignment: Qt.AlignCenter
        visible: !root.bitmap && !root.dotted && root.elevationShown
        text: root.elevationText
        textFormat: Text.PlainText
        color: root.color
        font.features: ({ "tnum": 1 })
        font.family: root.theme.digitsFont
        font.pixelSize: root.theme.fontSizeSmall
        opacity: 0.75
    }
}
