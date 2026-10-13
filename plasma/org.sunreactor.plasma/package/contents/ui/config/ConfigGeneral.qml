/*
 * Settings, with the panel item drawn live at the top: the checkboxes below
 * change the thing in the frame, so nobody has to guess what they do.
 */

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../Icons.js" as Icons

KCM.SimpleKCM {
    id: root

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    property alias cfg_socketPath: socketPathField.text
    property alias cfg_showTemperature: temperatureCheckBox.checked
    property alias cfg_showSolarElevation: elevationCheckBox.checked
    property int cfg_defaultSuspendMinutes: 60
    property int cfg_overrideDurationMinutes: 60
    property string cfg_themeMode: "follow"
    property string cfg_pinnedTheme: ""
    property alias cfg_accentOverride: accentField.text
    property string cfg_densityOverride: ""
    property alias cfg_pixelScale: pixelScaleBox.value

    // Plasma assigns these from main.xml so the Defaults button knows what to
    // restore. They exist to be written to, not read.
    property string cfg_socketPathDefault: ""
    property bool cfg_showTemperatureDefault: true
    property bool cfg_showSolarElevationDefault: false
    property int cfg_defaultSuspendMinutesDefault: 60
    property int cfg_overrideDurationMinutesDefault: 60
    property string cfg_themeModeDefault: "follow"
    property string cfg_pinnedThemeDefault: ""
    property string cfg_accentOverrideDefault: ""
    property string cfg_densityOverrideDefault: ""
    property int cfg_pixelScaleDefault: 0

    /*! Durations offered for both "set by hand" and "pause". */
    readonly property var durations: [
        { minutes: 30, label: i18ndp(root.domain, "%1 minute", "%1 minutes", 30) },
        { minutes: 60, label: i18ndp(root.domain, "%1 hour", "%1 hours", 1) },
        { minutes: 120, label: i18ndp(root.domain, "%1 hour", "%1 hours", 2) },
        { minutes: 240, label: i18ndp(root.domain, "%1 hour", "%1 hours", 4) },
        { minutes: 0, label: i18nd(root.domain, "Until I undo it") },
    ]

    // Its own connection: the settings window outlives no widget instance.
    SunReactorClient {
        id: preview
        socketPath: root.cfg_socketPath
        onThemeRegistryChanged: {
            if (themeRegistry === SunReactorClient.CapabilitySupported) {
                refreshThemeList();
            }
        }
    }

    readonly property bool themesAvailable: preview.themeRegistry === SunReactorClient.CapabilitySupported

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        // --- Live panel preview ------------------------------------------
        ColumnLayout {
            Layout.alignment: Qt.AlignHCenter
            Layout.bottomMargin: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.smallSpacing

            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.max(Kirigami.Units.gridUnit * 7,
                                                previewRow.implicitWidth + Kirigami.Units.gridUnit * 2)
                Layout.preferredHeight: Kirigami.Units.gridUnit * 2.6
                radius: Kirigami.Units.smallSpacing
                color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                               Kirigami.Theme.textColor.b, 0.07)
                border.width: 1
                border.color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                                      Kirigami.Theme.textColor.b, 0.12)

                RowLayout {
                    id: previewRow

                    anchors.centerIn: parent
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        // 32 is two screen pixels per art pixel, like a panel.
                        Layout.preferredWidth: 32
                        Layout.preferredHeight: 32
                        roundToIconSize: false
                        source: preview.hasWeatherReading
                            ? Icons.conditionArt(preview.weatherCondition, preview.weatherIsNight)
                            : (preview.isConnected ? Icons.skyArt(!preview.isDaylight)
                                                   : Icons.art("unknown"))
                    }

                    ColumnLayout {
                        spacing: 0
                        visible: temperatureCheckBox.checked || elevationCheckBox.checked

                        QQC2.Label {
                            Layout.alignment: Qt.AlignHCenter
                            visible: temperatureCheckBox.checked
                            text: preview.hasTemperature
                                ? i18ndc(root.domain, "Temperature in degrees, panel-sized", "%1°",
                                         Math.round(preview.temperatureC))
                                : i18ndc(root.domain, "No value is known yet", "—")
                            font.features: ({ "tnum": 1 })
                            font.pixelSize: elevationCheckBox.checked
                                ? Kirigami.Theme.smallFont.pixelSize
                                : Kirigami.Theme.defaultFont.pixelSize
                        }

                        QQC2.Label {
                            Layout.alignment: Qt.AlignHCenter
                            visible: elevationCheckBox.checked
                            text: preview.hasSolarElevation
                                ? i18ndc(root.domain, "Sun height above the horizon, panel-sized",
                                         "%1°", preview.solarElevation.toFixed(1))
                                : i18ndc(root.domain, "No value is known yet", "—")
                            font.features: ({ "tnum": 1 })
                            font.pixelSize: Kirigami.Theme.smallFont.pixelSize
                            opacity: 0.75
                        }
                    }
                }
            }

            QQC2.Label {
                Layout.alignment: Qt.AlignHCenter
                text: preview.isConnected
                    ? i18nd(root.domain, "How the widget looks in the panel right now")
                    : i18nd(root.domain, "Preview — the background service is not answering")
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }
        }

        // --- Settings -----------------------------------------------------
        Kirigami.FormLayout {
            Layout.fillWidth: true

            QQC2.CheckBox {
                id: temperatureCheckBox
                Kirigami.FormData.label: i18nd(root.domain, "In the panel:")
                text: i18nd(root.domain, "Show the outside temperature")
            }

            QQC2.CheckBox {
                id: elevationCheckBox
                text: i18nd(root.domain, "Show how high the sun stands")
            }

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "How long your changes last")
            }

            QQC2.ComboBox {
                id: overrideDurationBox
                Kirigami.FormData.label: i18nd(root.domain, "Brightness you set by hand:")
                model: root.durations
                textRole: "label"
                valueRole: "minutes"
                onActivated: root.cfg_overrideDurationMinutes = currentValue
                Component.onCompleted: currentIndex = indexOfValue(root.cfg_overrideDurationMinutes)
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "When the time is up, brightness follows the sun again.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            QQC2.ComboBox {
                id: suspendDurationBox
                Kirigami.FormData.label: i18nd(root.domain, "Pausing:")
                model: root.durations
                textRole: "label"
                valueRole: "minutes"
                onActivated: root.cfg_defaultSuspendMinutes = currentValue
                Component.onCompleted: currentIndex = indexOfValue(root.cfg_defaultSuspendMinutes)
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "Used by the Pause button and by middle-clicking the panel icon. In the popup you can also pause until sunrise or sunset.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "Look")
            }

            QQC2.ComboBox {
                id: themeModeBox
                Kirigami.FormData.label: i18nd(root.domain, "Theme:")
                model: [
                    { value: "follow", label: i18nd(root.domain, "Same as the terminal app") },
                    { value: "pinned", label: i18nd(root.domain, "Pick one for this widget") },
                ]
                textRole: "label"
                valueRole: "value"
                onActivated: root.cfg_themeMode = currentValue
                Component.onCompleted: currentIndex = Math.max(0, indexOfValue(root.cfg_themeMode))
            }

            QQC2.ComboBox {
                id: pinnedBox
                visible: root.cfg_themeMode === "pinned"
                enabled: root.themesAvailable
                Kirigami.FormData.label: i18nd(root.domain, "Pinned theme:")
                model: preview.themeList
                textRole: "name"
                valueRole: "id"
                displayText: currentIndex >= 0 ? currentText
                    : (root.cfg_pinnedTheme.length > 0 ? root.cfg_pinnedTheme : i18nd(root.domain, "Choose a theme"))
                onActivated: root.cfg_pinnedTheme = currentValue
                onCountChanged: currentIndex = indexOfValue(root.cfg_pinnedTheme)
                delegate: QQC2.ItemDelegate {
                    id: swatch
                    required property var modelData
                    required property int index
                    width: ListView.view ? ListView.view.width : implicitWidth
                    highlighted: pinnedBox.highlightedIndex === index
                    contentItem: RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        Repeater {
                            model: [swatch.modelData.bg, swatch.modelData.fg, swatch.modelData.accent,
                                    swatch.modelData.secondaryAccent]
                            Rectangle {
                                required property string modelData
                                implicitWidth: Kirigami.Units.iconSizes.small
                                implicitHeight: implicitWidth
                                color: modelData
                                border.width: 1
                                border.color: Qt.rgba(0.5, 0.5, 0.5, 0.5)
                            }
                        }
                        QQC2.Label {
                            Layout.fillWidth: true
                            text: swatch.modelData.name
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            QQC2.Label {
                visible: root.cfg_themeMode === "pinned" && !root.themesAvailable
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: preview.isConnected
                    ? i18nd(root.domain, "The background service is too old to offer other themes; the widget keeps following the terminal app's theme.")
                    : i18nd(root.domain, "The theme list appears once the background service answers.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            QQC2.TextField {
                id: accentField
                Kirigami.FormData.label: i18nd(root.domain, "Accent colour:")
                placeholderText: i18nd(root.domain, "Theme default, or #rrggbb")
                validator: RegularExpressionValidator { regularExpression: /^(#[0-9a-fA-F]{6})?$/ }
            }

            QQC2.ComboBox {
                id: densityBox
                Kirigami.FormData.label: i18nd(root.domain, "Density:")
                model: [
                    { value: "", label: i18nd(root.domain, "Theme default") },
                    { value: "compact", label: i18nd(root.domain, "Compact") },
                    { value: "comfortable", label: i18nd(root.domain, "Comfortable") },
                ]
                textRole: "label"
                valueRole: "value"
                onActivated: root.cfg_densityOverride = currentValue
                Component.onCompleted: currentIndex = Math.max(0, indexOfValue(root.cfg_densityOverride))
            }

            QQC2.SpinBox {
                id: pixelScaleBox
                Kirigami.FormData.label: i18nd(root.domain, "Pixel size:")
                from: 0
                to: 6
                textFromValue: (value, locale) => value === 0
                    ? i18ndc(root.domain, "Pixel size chosen from the screen scale", "Automatic")
                    : i18ndp(root.domain, "%1 px", "%1 px", value)
                valueFromText: (text, locale) => {
                    const n = parseInt(text);
                    return isNaN(n) ? 0 : n;
                }
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "Pixel themes draw every art pixel as a whole block of screen pixels, so they stay sharp at any display scale.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "Advanced")
            }

            QQC2.TextField {
                id: socketPathField
                Kirigami.FormData.label: i18nd(root.domain, "Service socket:")
                placeholderText: i18nd(root.domain, "Leave empty to use the default location")
                Layout.fillWidth: true
            }
        }
    }
}
