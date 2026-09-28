/*
 * Today's three turning points, with the day drawn as a track underneath so
 * the current moment has a place. The next event is emphasised; the sun's
 * height is only spelled out when the user asked for it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../Format.js" as Format
import "../wardrobe"

PlasmaComponents3.ItemDelegate {
    id: root

    required property SunReactorClient client
    property var tokens: null

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property var entries: [
        { label: i18nd(root.domain, "Sunrise"), epoch: client.sunriseEpochS },
        { label: i18nd(root.domain, "Solar noon"), epoch: client.solarNoonEpochS },
        { label: i18nd(root.domain, "Sunset"), epoch: client.sunsetEpochS },
    ]
    /*! Epoch of the next event still ahead today, or 0 once the day is done. */
    readonly property real nextEpoch: {
        for (const entry of entries) {
            if (entry.epoch > client.nowEpochS) {
                return entry.epoch;
            }
        }
        return 0;
    }
    /*! How far the day has run, 0 at sunrise and 1 at sunset. */
    readonly property real dayProgress: {
        const rise = client.sunriseEpochS;
        const set = client.sunsetEpochS;
        if (rise <= 0 || set <= rise) {
            return 0;
        }
        return Math.max(0, Math.min(1, (client.nowEpochS - rise) / (set - rise)));
    }
    readonly property bool daylight: client.nowEpochS >= client.sunriseEpochS
        && client.nowEpochS < client.sunsetEpochS
    /*! Active theme accent color or desktop highlight fallback. */
    readonly property color accentColor: (tokens && tokens.accentColor)
        ? tokens.accentColor
        : ((client.themeAccent && client.themeAccent.length > 0) ? client.themeAccent : Kirigami.Theme.highlightColor)
    /*! Set by the popup from the widget's settings; off unless asked for. */
    property bool showSolarElevation: false
    readonly property bool showElevation: showSolarElevation && client.hasSolarElevation

    Layout.fillWidth: true
    visible: client.sunriseEpochS > 0 && client.sunsetEpochS > 0

    background.visible: false
    hoverEnabled: false
    Accessible.ignored: true

    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.gridUnit

            Repeater {
                model: root.entries

                delegate: ColumnLayout {
                    id: entry

                    required property var modelData

                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 0

                    PlasmaExtras.DescriptiveLabel {
                        Layout.fillWidth: true
                        text: entry.modelData.label
                        textFormat: Text.PlainText
                        font.family: (root.tokens && root.tokens.fontFamily) || ""
                        color: (root.tokens && root.tokens.textMutedColor) ? root.tokens.textMutedColor : Kirigami.Theme.disabledTextColor
                        elide: Text.ElideRight
                        horizontalAlignment: Text.AlignHCenter
                    }

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: Format.timeText(entry.modelData.epoch)
                        textFormat: Text.PlainText
                        horizontalAlignment: Text.AlignHCenter
                        font.features: ({ "tnum": 1 })
                        font.family: (root.tokens && root.tokens.digitFontFamily) || ""
                        font.pixelSize: Math.round(Kirigami.Theme.defaultFont.pixelSize * (root.tokens ? root.tokens.digitFontScale : 1.0))
                        font.weight: entry.modelData.epoch === root.nextEpoch ? Font.Bold : Font.Normal
                        color: entry.modelData.epoch === root.nextEpoch ? root.accentColor : ((root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor)
                    }
                }
            }
        }

        // Thematic solar track: dot-matrix, 4-shade pixel, Amiga Boing Ball, VFD phosphor, or neon
        ThemedSolarTrack {
            Layout.fillWidth: true
            tokens: root.tokens
            dayProgress: root.dayProgress
            daylight: root.daylight
            accentColor: root.accentColor
        }

        PlasmaExtras.DescriptiveLabel {
            Layout.fillWidth: true
            visible: root.showElevation
            horizontalAlignment: Text.AlignHCenter
            textFormat: Text.PlainText
            font.family: (root.tokens && root.tokens.fontFamily) || ""
            text: root.client.solarElevation >= 0
                ? i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° above the horizon", root.client.solarElevation.toFixed(1))
                : i18ndc(root.domain, "Placeholder is an angle in degrees",
                         "Sun %1° below the horizon", (-root.client.solarElevation).toFixed(1))
        }
    }
}
