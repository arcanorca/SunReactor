/*
 * Settings, with the panel item drawn live at the top: the choices below
 * change the thing in the frame, so nobody has to guess what they do.
 * Sections: what the panel shows, how long changes last, the look of the
 * popup (theme, colours, sections) and where your own themes live.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kcmutils as KCM
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../modules/Modules.js" as Modules
import "../components"
import "../components/weathericons"
import "../engine"

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
    property int cfg_uiScale: 0
    property int cfg_pixelScale: 0
    property string cfg_rendererOverride: ""
    property string cfg_weatherIconStyle: "auto"
    property var cfg_moduleOrder: []
    property var cfg_hiddenModules: []
    property string cfg_panelMode: "sky"
    property string cfg_panelDisplay: ""

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
    property int cfg_uiScaleDefault: 0
    property int cfg_pixelScaleDefault: 0
    property string cfg_rendererOverrideDefault: ""
    property string cfg_weatherIconStyleDefault: "auto"
    property var cfg_moduleOrderDefault: []
    property var cfg_hiddenModulesDefault: ["telemetry"]
    property string cfg_panelModeDefault: "sky"
    property string cfg_panelDisplayDefault: ""

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
        pinnedThemeId: root.cfg_themeMode === "pinned" ? root.cfg_pinnedTheme : ""
        onThemeRegistryChanged: {
            if (themeRegistry === SunReactorClient.CapabilitySupported) {
                refreshThemeList();
            }
        }
    }

    // The unsaved look, resolved the way the widget resolves it, for the preview.
    Theme {
        id: previewTheme
        spec: root.cfg_themeMode === "pinned" && preview.pinnedTheme && preview.pinnedTheme.palette !== undefined
            ? preview.pinnedTheme : preview.theme
        overrides: ({
            accent: root.cfg_accentOverride,
            renderer: root.cfg_rendererOverride,
            weatherIconStyle: root.cfg_weatherIconStyle,
            uiScale: root.cfg_uiScale,
        })
    }

    /*! The condition the preview icons show: the client's reading, else clear (or unknown offline). */
    readonly property string previewCondition: preview.hasWeatherReading ? preview.weatherCondition
                                                                         : (preview.isConnected ? "clear" : "")
    readonly property bool previewNight: preview.hasWeatherReading ? preview.weatherIsNight : !preview.isDaylight

    readonly property bool themesAvailable: preview.themeRegistry === SunReactorClient.CapabilitySupported
    readonly property bool validAccent: /^#[0-9a-fA-F]{6}$/.test(cfg_accentOverride)

    /*! Display names; a string so the list below is rebuilt only when it changes
        (the status arrives every few seconds). */
    readonly property string monitorKey: preview.monitors.map(m => m.logicalId).join("\n")
    readonly property var monitorIds: monitorKey.length > 0 ? monitorKey.split("\n") : []

    /*! The themes, by name. */
    readonly property var sortedThemes: preview.themeList.slice().sort(
        (a, b) => String(a.name).localeCompare(String(b.name)))

    // --- popup sections: visibility and order ----------------------------------------

    /*! The status line sits above the tab bar, whatever the order: it has a
        checkbox of its own and stays out of the section list. */
    readonly property string statusId: "telemetry"
    readonly property var allModules: Object.keys(Modules.FILES)
    readonly property var sectionIds: allModules.filter(id => id !== statusId)
    readonly property var moduleLabels: ({
        curve: i18ndc(root.domain, "Popup section", "Light cycle"),
        displays: i18ndc(root.domain, "Popup section", "Displays"),
        automation: i18ndc(root.domain, "Popup section", "Automation"),
        weather: i18ndc(root.domain, "Popup section", "Weather"),
        forecast: i18ndc(root.domain, "Popup section", "Forecast"),
        suntimes: i18ndc(root.domain, "Popup section", "Sun times")
    })
    /*! The sections the theme itself shows, in its order. */
    readonly property var themeModules: {
        const theme = preview.pinnedThemeId.length > 0 && preview.pinnedTheme.widget ? preview.pinnedTheme : preview.theme;
        const list = theme && theme.widget ? theme.widget.modules : null;
        return list && list.length > 0 ? Array.from(list) : allModules;
    }
    /*! The order in effect: the chosen one, else the theme's. */
    readonly property var effectiveOrder: Array.from(cfg_moduleOrder.length > 0 ? cfg_moduleOrder : themeModules)
    /*! The status line is in the order (the theme's or the chosen one) ... */
    readonly property bool statusLineInOrder: effectiveOrder.indexOf(statusId) >= 0
    /*! ... and not hidden: the popup shows it above the tabs. */
    readonly property bool statusLineShown: statusLineInOrder && cfg_hiddenModules.indexOf(statusId) < 0
    /*! Rows of the section list: those in the order first, then the rest (off). */
    readonly property var moduleRows: {
        const hidden = Array.from(cfg_hiddenModules);
        const order = [];
        for (const id of effectiveOrder) {
            if (sectionIds.indexOf(id) >= 0 && order.indexOf(id) < 0) {
                order.push(id);
            }
        }
        const rows = order.map((id, pos) => ({ id: id, label: moduleLabels[id], inOrder: true, pos: pos,
                                               count: order.length, shown: hidden.indexOf(id) < 0 }));
        for (const id of sectionIds) {
            if (order.indexOf(id) < 0) {
                rows.push({ id: id, label: moduleLabels[id], inOrder: false, pos: -1, count: order.length,
                            shown: false });
            }
        }
        return rows;
    }
    readonly property bool modulesCustomised: cfg_moduleOrder.length > 0
        || cfg_hiddenModules.join("\n") !== cfg_hiddenModulesDefault.join("\n")

    function orderedIds() {
        return moduleRows.filter(r => r.inOrder).map(r => r.id);
    }

    /*! Shows or hides a section. Switching a section on that the theme leaves
        out makes the order explicit, with the section at the end. */
    function toggleModule(id) {
        const row = moduleRows.find(r => r.id === id);
        if (!row) {
            return;
        }
        const hidden = Array.from(cfg_hiddenModules).filter(h => h !== id);
        if (!row.inOrder) {
            cfg_moduleOrder = orderedIds().concat([id]);
        } else if (row.shown) {
            hidden.push(id);
        }
        cfg_hiddenModules = hidden;
    }

    /*! Shows or hides the status line. Showing it when the order leaves it out
        puts it first in the order. */
    function setStatusLine(on) {
        const hidden = Array.from(cfg_hiddenModules).filter(h => h !== statusId);
        if (on) {
            if (!statusLineInOrder) {
                cfg_moduleOrder = [statusId].concat(orderedIds());
            }
        } else {
            hidden.push(statusId);
        }
        cfg_hiddenModules = hidden;
    }

    function moveModule(id, delta) {
        const order = orderedIds();
        const at = order.indexOf(id);
        const to = at + delta;
        if (at < 0 || to < 0 || to >= order.length) {
            return;
        }
        order.splice(to, 0, order.splice(at, 1)[0]);
        // The stored order keeps the status line in it, first, when it is in use.
        cfg_moduleOrder = (statusLineInOrder ? [statusId] : []).concat(order);
    }

    function resetModules() {
        cfg_moduleOrder = [];
        cfg_hiddenModules = Array.from(cfg_hiddenModulesDefault);
    }

    property string folderNote: ""

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
                Layout.preferredHeight: previewTheme.gridUnit * 2.6
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

                    // The current condition in the chosen style, at a panel size and a popup size.
                    ThemedWeatherIcon {
                        Layout.alignment: Qt.AlignVCenter
                        theme: previewTheme
                        condition: root.previewCondition
                        night: root.previewNight
                        size: Math.round(32 * previewTheme.scaleFactor)
                        surface: Kirigami.Theme.backgroundColor
                        ink: Kirigami.Theme.textColor
                    }

                    ThemedWeatherIcon {
                        Layout.alignment: Qt.AlignVCenter
                        theme: previewTheme
                        condition: root.previewCondition
                        night: root.previewNight
                        size: Math.round(22 * previewTheme.scaleFactor)
                        surface: Kirigami.Theme.backgroundColor
                        ink: Kirigami.Theme.textColor
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
                                ? previewTheme.fontSizeSmall
                                : previewTheme.fontSizeBody
                        }

                        QQC2.Label {
                            Layout.alignment: Qt.AlignHCenter
                            visible: elevationCheckBox.checked
                            text: preview.hasSolarElevation
                                ? i18ndc(root.domain, "Sun height above the horizon, panel-sized",
                                         "%1°", preview.solarElevation.toFixed(1))
                                : i18ndc(root.domain, "No value is known yet", "—")
                            font.features: ({ "tnum": 1 })
                            font.pixelSize: previewTheme.fontSizeSmall
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

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "In the panel")
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Show:")
                configValue: root.cfg_panelMode
                model: [
                    { value: "sky", label: i18ndc(root.domain, "Panel item", "Weather picture") },
                    { value: "readout", label: i18ndc(root.domain, "Panel item", "Brightness number") },
                    { value: "sparkline", label: i18ndc(root.domain, "Panel item", "Today's brightness curve") },
                    { value: "ring", label: i18ndc(root.domain, "Panel item", "Brightness ring") },
                ]
                onValueSelected: value => root.cfg_panelMode = value
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Display:")
                enabled: root.cfg_panelMode !== "sky"
                configValue: root.cfg_panelDisplay
                model: [{ value: "", label: i18nd(root.domain, "All displays together") }].concat(
                    root.monitorIds.map(id => ({ value: id, label: id })))
                onValueSelected: value => root.cfg_panelDisplay = value
            }

            QQC2.CheckBox {
                id: temperatureCheckBox
                Kirigami.FormData.label: i18nd(root.domain, "Also show:")
                text: i18nd(root.domain, "The outside temperature")
            }

            QQC2.CheckBox {
                id: elevationCheckBox
                text: i18nd(root.domain, "How high the sun stands")
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

            Kirigami.InlineMessage {
                Layout.fillWidth: true
                Layout.maximumWidth: Kirigami.Units.gridUnit * 24
                visible: preview.themeWarning.length > 0
                type: Kirigami.MessageType.Warning
                text: preview.themeWarning
            }

            QQC2.ButtonGroup { id: themeModeGroup }

            QQC2.RadioButton {
                id: followRadio
                Kirigami.FormData.label: i18nd(root.domain, "Theme:")
                QQC2.ButtonGroup.group: themeModeGroup
                text: i18nd(root.domain, "Follow SunReactor")
                onClicked: root.cfg_themeMode = "follow"

                Binding {
                    target: followRadio
                    property: "checked"
                    value: root.cfg_themeMode !== "pinned"
                }
            }

            QQC2.RadioButton {
                id: pinRadio
                QQC2.ButtonGroup.group: themeModeGroup
                text: i18nd(root.domain, "Pick one for this widget")
                onClicked: root.cfg_themeMode = "pinned"

                Binding {
                    target: pinRadio
                    property: "checked"
                    value: root.cfg_themeMode === "pinned"
                }
            }

            Item {
                visible: root.cfg_themeMode === "pinned" && root.themesAvailable
                implicitWidth: Kirigami.Units.gridUnit * 18
                implicitHeight: Kirigami.Units.gridUnit * 11

                Rectangle {
                    anchors.fill: parent
                    color: Kirigami.Theme.backgroundColor
                    border.width: 1
                    border.color: Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                                          Kirigami.Theme.textColor.b, 0.25)
                    radius: Kirigami.Units.smallSpacing
                }

                ListView {
                    id: themeListItems

                    anchors.fill: parent
                    anchors.margins: 1
                    clip: true
                    model: root.sortedThemes
                    currentIndex: -1
                    boundsBehavior: Flickable.StopAtBounds
                    // Open on the pinned theme instead of the top of the list.
                    onCountChanged: positionViewAtIndex(
                        Math.max(0, root.sortedThemes.findIndex(t => t.id === root.cfg_pinnedTheme)),
                        ListView.Contain)
                    QQC2.ScrollBar.vertical: QQC2.ScrollBar {}

                    delegate: QQC2.ItemDelegate {
                        id: themeRow

                        required property var modelData
                        required property int index

                        width: ListView.view ? ListView.view.width : implicitWidth
                        highlighted: themeRow.modelData.id === root.cfg_pinnedTheme
                        onClicked: root.cfg_pinnedTheme = themeRow.modelData.id

                        Accessible.name: modelData.name
                        Accessible.description: highlighted ? i18nd(root.domain, "Selected") : ""

                        contentItem: RowLayout {
                            spacing: Kirigami.Units.smallSpacing

                            ThemeSwatches {
                                colors: [themeRow.modelData.bg, themeRow.modelData.fg, themeRow.modelData.accent,
                                         themeRow.modelData.secondaryAccent]
                            }
                            QQC2.Label {
                                Layout.fillWidth: true
                                text: themeRow.modelData.name
                                elide: Text.ElideRight
                            }
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

            RowLayout {
                Kirigami.FormData.label: i18nd(root.domain, "Accent colour:")
                spacing: Kirigami.Units.smallSpacing

                Rectangle {
                    visible: root.validAccent
                    implicitWidth: Kirigami.Units.iconSizes.smallMedium
                    implicitHeight: implicitWidth
                    color: root.validAccent ? root.cfg_accentOverride : "transparent"
                    border.width: 1
                    border.color: Qt.rgba(0.5, 0.5, 0.5, 0.55)
                    Accessible.ignored: true
                }

                QQC2.TextField {
                    id: accentField
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                    placeholderText: i18nd(root.domain, "Theme default, or #rrggbb")
                    validator: RegularExpressionValidator { regularExpression: /^(#[0-9a-fA-F]{6})?$/ }
                }

                QQC2.ToolButton {
                    visible: accentField.text.length > 0
                    icon.name: "edit-clear"
                    text: i18nd(root.domain, "Use the theme's accent")
                    display: QQC2.AbstractButton.IconOnly
                    onClicked: accentField.text = ""

                    QQC2.ToolTip.visible: hovered
                    QQC2.ToolTip.text: text
                }
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Density:")
                configValue: root.cfg_densityOverride
                model: [
                    { value: "", label: i18nd(root.domain, "Theme default") },
                    { value: "compact", label: i18nd(root.domain, "Compact") },
                    { value: "comfortable", label: i18nd(root.domain, "Comfortable") },
                ]
                onValueSelected: value => root.cfg_densityOverride = value
            }

            ValueComboBox {
                id: scaleCombo
                Kirigami.FormData.label: i18nd(root.domain, "Interface scale:")
                configValue: root.cfg_uiScale
                model: [
                    { value: 0, label: i18ndc(root.domain, "Scale option", "Automatic (screen-based)") },
                    { value: 85, label: "85%" },
                    { value: 100, label: i18ndc(root.domain, "Scale option", "100% (Standard)") },
                    { value: 115, label: "115%" },
                    { value: 125, label: "125%" },
                    { value: 135, label: "135%" },
                    { value: 150, label: "150%" },
                    { value: 175, label: "175%" },
                    { value: 200, label: "200%" },
                ]
                onValueSelected: value => root.cfg_uiScale = value
            }

            RowLayout {
                Kirigami.FormData.label: i18nd(root.domain, "Fine scale adjustment:")
                spacing: Kirigami.Units.smallSpacing

                QQC2.Slider {
                    id: customScaleSlider
                    Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                    from: 50
                    to: 250
                    stepSize: 5
                    value: root.cfg_uiScale > 0 ? root.cfg_uiScale : Math.round(previewTheme.scaleFactor * 100)
                    onMoved: root.cfg_uiScale = Math.round(value)
                }

                QQC2.Label {
                    text: root.cfg_uiScale === 0
                        ? i18ndc(root.domain, "Scale status", "Auto (%1%)", Math.round(previewTheme.scaleFactor * 100))
                        : i18ndc(root.domain, "Percentage scale", "%1%", Math.round(customScaleSlider.value))
                    font: Kirigami.Theme.smallFont
                }

                QQC2.ToolButton {
                    visible: root.cfg_uiScale !== 0
                    icon.name: "edit-clear"
                    text: i18nd(root.domain, "Reset to automatic scale")
                    display: QQC2.AbstractButton.IconOnly
                    onClicked: root.cfg_uiScale = 0

                    QQC2.ToolTip.visible: hovered
                    QQC2.ToolTip.text: text
                }
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "Scales text, icons, and layout across all themes. Automatic dynamically selects the ideal size for your screen resolution and density.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Drawing style:")
                configValue: root.cfg_rendererOverride
                model: [
                    { value: "", label: i18nd(root.domain, "Theme default") },
                    { value: "pixel", label: i18ndc(root.domain, "Drawing style", "Pixel art") },
                    { value: "vector", label: i18ndc(root.domain, "Drawing style", "Instrument panel") },
                    { value: "terminal", label: i18ndc(root.domain, "Drawing style", "Terminal text") },
                ]
                onValueSelected: value => root.cfg_rendererOverride = value
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Pixel size:")
                configValue: root.cfg_pixelScale
                model: [0, 1, 2, 3, 4, 5, 6].map(n => ({
                    value: n,
                    label: n === 0 ? i18ndc(root.domain, "Pixel size chosen from the screen scale", "Automatic")
                                   : i18ndp(root.domain, "%1 pixel", "%1 pixels", n)
                }))
                onValueSelected: value => root.cfg_pixelScale = value
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "Pixel themes draw every art pixel as a whole block of screen pixels, so they stay sharp at any display scale.")
                wrapMode: Text.WordWrap
                font: Kirigami.Theme.smallFont
                opacity: 0.75
            }

            ValueComboBox {
                Kirigami.FormData.label: i18nd(root.domain, "Weather icons:")
                configValue: root.cfg_weatherIconStyle
                model: [
                    { value: "auto", label: i18ndc(root.domain, "Weather icon style", "Match the theme") },
                    { value: "modern", label: i18ndc(root.domain, "Weather icon style", "Modern (colour)") },
                    { value: "line", label: i18ndc(root.domain, "Weather icon style", "Outline") },
                    { value: "solid", label: i18ndc(root.domain, "Weather icon style", "Filled") },
                    { value: "pixel", label: i18ndc(root.domain, "Weather icon style", "Pixel art") },
                ]
                onValueSelected: value => root.cfg_weatherIconStyle = value
            }

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "Popup sections")
            }

            QQC2.CheckBox {
                Kirigami.FormData.label: i18nd(root.domain, "Status line:")
                text: i18nd(root.domain, "Show the status line at the top of the popup")
                checked: root.statusLineShown
                onToggled: root.setStatusLine(checked)
            }

            ColumnLayout {
                Kirigami.FormData.label: i18nd(root.domain, "Show and arrange:")
                Kirigami.FormData.labelAlignment: Qt.AlignTop
                spacing: 0

                // A fixed number of rows that follow `moduleRows` by position:
                // rebuilding the delegates while the form lays out crashes Qt Quick Layouts.
                Repeater {
                    model: root.sectionIds.length

                    delegate: RowLayout {
                        id: moduleRow

                        required property int index
                        readonly property var modelData: root.moduleRows[index]

                        Layout.minimumWidth: Kirigami.Units.gridUnit * 16
                        spacing: Kirigami.Units.smallSpacing

                        QQC2.CheckBox {
                            Layout.fillWidth: true
                            text: moduleRow.modelData.label
                            checked: moduleRow.modelData.shown
                            onToggled: root.toggleModule(moduleRow.modelData.id)
                        }

                        QQC2.ToolButton {
                            enabled: moduleRow.modelData.inOrder && moduleRow.modelData.pos > 0
                            icon.name: "go-up"
                            text: i18nd(root.domain, "Move up")
                            display: QQC2.AbstractButton.IconOnly
                            onClicked: root.moveModule(moduleRow.modelData.id, -1)

                            QQC2.ToolTip.visible: hovered
                            QQC2.ToolTip.text: i18nd(root.domain, "Move %1 up", moduleRow.modelData.label)
                        }

                        QQC2.ToolButton {
                            enabled: moduleRow.modelData.inOrder
                                && moduleRow.modelData.pos < moduleRow.modelData.count - 1
                            icon.name: "go-down"
                            text: i18nd(root.domain, "Move down")
                            display: QQC2.AbstractButton.IconOnly
                            onClicked: root.moveModule(moduleRow.modelData.id, 1)

                            QQC2.ToolTip.visible: hovered
                            QQC2.ToolTip.text: i18nd(root.domain, "Move %1 down", moduleRow.modelData.label)
                        }
                    }
                }
            }

            QQC2.Button {
                visible: root.modulesCustomised
                icon.name: "edit-undo"
                text: i18nd(root.domain, "Use the theme's sections")
                onClicked: root.resetModules()
            }

            Item {
                Kirigami.FormData.isSection: true
                Kirigami.FormData.label: i18nd(root.domain, "Your own themes")
            }

            QQC2.Label {
                Layout.maximumWidth: Kirigami.Units.gridUnit * 18
                text: i18nd(root.domain, "Your own themes are text files in a folder in your home directory, one file per theme. Copy a built-in theme there and change its colours to make your own; it shows up in the list once the background service reloads its settings.")
                wrapMode: Text.WordWrap
            }

            QQC2.Button {
                icon.name: "folder-open"
                text: i18nd(root.domain, "Open themes folder")
                onClicked: root.folderNote = preview.openThemesFolder()
                    ? "" : i18nd(root.domain, "The themes folder could not be opened.")
            }

            QQC2.Label {
                visible: root.folderNote.length > 0
                text: root.folderNote
                color: Kirigami.Theme.negativeTextColor
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
