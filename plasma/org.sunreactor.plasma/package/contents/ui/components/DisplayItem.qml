/*
 * One brightness row: icon, name, state, value and slider. Used both for a
 * single display and for the "All displays" master row, so it knows nothing
 * about monitors or the daemon - it reports the value the user asked for and
 * lets the popup decide what that means.
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami

import "../wardrobe"
import "../wardrobe/amiga"

PlasmaComponents3.ItemDelegate {
    id: root

    /*! Percentage to display, or -1 when the daemon has no value for it. */
    required property int percent
    property string iconName: "video-display-brightness"
    /*! Short state note shown before the value, e.g. "Manual". */
    property string hint: ""
    /*! False disables the slider: a display that is off, or not responding. */
    property bool controllable: true
    property color accentColor: tokens ? tokens.accentColor : Kirigami.Theme.highlightColor

    property int minPct: 15
    property int maxPct: 60
    property bool hasLimits: false
    property bool hasOverride: false
    property bool limitsExpanded: false

    property var tokens: null

    /*! Emitted while dragging (debounced) and once on release. */
    signal requested(int percent)
    signal limitsRequested(int minPct, int maxPct)
    signal clearOverrideRequested()

    readonly property bool hasValue: percent >= 0
    readonly property string valueText: hasValue
        ? i18ndc("plasma_applet_org.sunreactor.plasma", "Brightness percentage", "%1%", percent)
        : i18ndc("plasma_applet_org.sunreactor.plasma", "No value is known yet", "—")

    Layout.fillWidth: true

    background.visible: highlighted
    highlighted: activeFocus
    hoverEnabled: false
    Accessible.ignored: true
    Keys.forwardTo: [slider]

    // Arrow keys move between rows; the slider itself uses left and right.
    Keys.onUpPressed: {
        const previous = nextItemInFocusChain(false);
        if (previous) {
            previous.forceActiveFocus(Qt.BacktabFocusReason);
        }
    }
    Keys.onDownPressed: {
        const next = nextItemInFocusChain(true);
        if (next) {
            next.forceActiveFocus(Qt.TabFocusReason);
        }
    }

    // DDC/CI writes travel over an I2C bus that does not like packet storms,
    // so a drag sends one request every quarter second instead of one per pixel.
    Timer {
        id: throttle
        interval: 250
        onTriggered: root.requested(Math.round(slider.value))
    }

    Timer {
        id: limitsThrottle
        interval: 250
        onTriggered: root.limitsRequested(Math.round(minSlider.value), Math.round(maxSlider.value))
    }

    contentItem: RowLayout {
        spacing: Kirigami.Units.gridUnit

        Kirigami.Icon {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            source: root.iconName
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: Kirigami.Units.smallSpacing

            // Guru Meditation Alert banner for unreachable monitors on Amiga theme
            GuruMeditationAlert {
                Layout.fillWidth: true
                visible: !root.controllable && root.tokens && root.tokens.guruMeditationStyle
                logicalId: root.text
                fontName: (root.tokens && root.tokens.digitFontFamily) ? root.tokens.digitFontFamily : "VT323, monospace"
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.text
                    textFormat: Text.PlainText
                    font.family: (root.tokens && root.tokens.fontFamily) || ""
                    color: (root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor
                    elide: Text.ElideRight
                }

                // If in manual override, provide instant one-click return to solar curve
                PlasmaComponents3.Button {
                    visible: root.hasOverride
                    icon.name: "edit-undo"
                    text: i18ndc("plasma_applet_org.sunreactor.plasma", "Return to automatic solar tracking", "Auto")
                    display: PlasmaComponents3.AbstractButton.TextBesideIcon
                    font.family: (root.tokens && root.tokens.fontFamily) || ""
                    onClicked: root.clearOverrideRequested()
                }

                PlasmaExtras.DescriptiveLabel {
                    text: root.hint
                    textFormat: Text.PlainText
                    font.family: (root.tokens && root.tokens.fontFamily) || ""
                    color: (root.tokens && root.tokens.textMutedColor) ? root.tokens.textMutedColor : Kirigami.Theme.disabledTextColor
                    visible: root.hint.length > 0 && !root.hasOverride && !(root.tokens && root.tokens.guruMeditationStyle && !root.controllable)
                }

                // Limits toggle button showing current limits range
                PlasmaComponents3.Button {
                    visible: root.hasLimits
                    flat: true
                    checkable: true
                    checked: root.limitsExpanded
                    icon.name: root.limitsExpanded ? "arrow-up" : "configure"
                    text: i18ndc("plasma_applet_org.sunreactor.plasma", "Range limits", "%1%–%2%", root.minPct, root.maxPct)
                    display: PlasmaComponents3.AbstractButton.TextBesideIcon
                    font.family: (root.tokens && root.tokens.digitFontFamily) || ""
                    onClicked: root.limitsExpanded = !root.limitsExpanded
                }

                PlasmaComponents3.Label {
                    text: root.valueText
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                    font.family: (root.tokens && root.tokens.digitFontFamily) || ""
                    font.pixelSize: Math.round(Kirigami.Theme.defaultFont.pixelSize * (root.tokens ? root.tokens.digitFontScale : 1.0))
                    color: root.hasValue && (slider.pressed || root.activeFocus)
                        ? (root.tokens ? root.tokens.accentColor : root.accentColor)
                        : ((root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor)
                }
            }

            ThemedSlider {
                id: slider

                tokens: root.tokens
                customAccent: root.tokens ? root.tokens.accentColor : root.accentColor
                Layout.fillWidth: true
                from: 0
                to: 100
                stepSize: 1
                enabled: root.controllable && root.hasValue
                value: root.hasValue ? root.percent : 0
                activeFocusOnTab: false

                // While dragging, ignore status updates: the daemon reports the
                // value it has written, which lags behind the thumb.
                onPressedChanged: {
                    if (pressed) {
                        return;
                    }
                    throttle.stop();
                    root.requested(Math.round(value));
                    value = Qt.binding(() => root.hasValue ? root.percent : 0);
                }

                onMoved: throttle.restart()

                Accessible.name: root.text
                Accessible.description: root.valueText
            }

            // Expandable GNOME HIG Limits Section
            ColumnLayout {
                Layout.fillWidth: true
                visible: root.hasLimits && root.limitsExpanded
                spacing: Kirigami.Units.smallSpacing

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: 1
                    color: root.tokens ? root.tokens.cardBorderColor : Qt.rgba(Kirigami.Theme.textColor.r, Kirigami.Theme.textColor.g,
                                                                               Kirigami.Theme.textColor.b, 0.08)
                }

                // Night Minimum Brightness row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        source: "weather-clear-night"
                    }

                    PlasmaComponents3.Label {
                        text: i18ndc("plasma_applet_org.sunreactor.plasma", "Night minimum boundary", "Night Min:")
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                        font.family: (root.tokens && root.tokens.fontFamily) || ""
                        color: (root.tokens && root.tokens.textMutedColor) ? root.tokens.textMutedColor : Kirigami.Theme.disabledTextColor
                    }

                    ThemedSlider {
                        id: minSlider
                        tokens: root.tokens
                        customAccent: root.tokens ? root.tokens.accentColor : root.accentColor
                        Layout.fillWidth: true
                        from: 0
                        to: Math.min(100, Math.round(maxSlider.value))
                        stepSize: 1
                        value: root.minPct
                        onMoved: limitsThrottle.restart()
                        onPressedChanged: {
                            if (!pressed) {
                                limitsThrottle.stop();
                                root.limitsRequested(Math.round(minSlider.value), Math.round(maxSlider.value));
                            }
                        }
                    }

                    PlasmaComponents3.Label {
                        text: i18ndc("plasma_applet_org.sunreactor.plasma", "Percentage", "%1%", Math.round(minSlider.value))
                        font.features: ({ "tnum": 1 })
                        font.family: (root.tokens && root.tokens.digitFontFamily) || ""
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                        color: (root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                        horizontalAlignment: Text.AlignRight
                    }
                }

                // Day Maximum Brightness row
                RowLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        Layout.preferredWidth: Kirigami.Units.iconSizes.small
                        Layout.preferredHeight: Kirigami.Units.iconSizes.small
                        source: "weather-clear"
                    }

                    PlasmaComponents3.Label {
                        text: i18ndc("plasma_applet_org.sunreactor.plasma", "Day maximum boundary", "Day Max:")
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                        font.family: (root.tokens && root.tokens.fontFamily) || ""
                        color: (root.tokens && root.tokens.textMutedColor) ? root.tokens.textMutedColor : Kirigami.Theme.disabledTextColor
                    }

                    ThemedSlider {
                        id: maxSlider
                        tokens: root.tokens
                        customAccent: root.tokens ? root.tokens.accentColor : root.accentColor
                        Layout.fillWidth: true
                        from: Math.max(0, Math.round(minSlider.value))
                        to: 100
                        stepSize: 1
                        value: root.maxPct
                        onMoved: limitsThrottle.restart()
                        onPressedChanged: {
                            if (!pressed) {
                                limitsThrottle.stop();
                                root.limitsRequested(Math.round(minSlider.value), Math.round(maxSlider.value));
                            }
                        }
                    }

                    PlasmaComponents3.Label {
                        text: i18ndc("plasma_applet_org.sunreactor.plasma", "Percentage", "%1%", Math.round(maxSlider.value))
                        font.features: ({ "tnum": 1 })
                        font.family: (root.tokens && root.tokens.digitFontFamily) || ""
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                        color: (root.tokens && root.tokens.textColor) ? root.tokens.textColor : Kirigami.Theme.textColor
                        Layout.preferredWidth: Kirigami.Units.gridUnit * 2
                        horizontalAlignment: Text.AlignRight
                    }
                }
            }
        }
    }
}
