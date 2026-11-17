/*
 * One brightness row: icon, name, state, value and slider. Used both for a
 * single display and for the "All displays" master row, so it knows nothing
 * about monitors or the daemon - it reports the value the user asked for and
 * lets the Displays module decide what that means.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami

import "../engine"

PlasmaComponents3.ItemDelegate {
    id: root

    required property Theme theme
    /*! Percentage to display, or -1 when the daemon has no value for it. */
    required property int percent
    property string iconName: "video-display-brightness"
    /*! Short state note shown before the value, e.g. "Manual". */
    property string hint: ""
    /*! False disables the slider: a display that is off, or not responding. */
    property bool controllable: true

    property int minPct: 15
    property int maxPct: 60
    property bool hasLimits: false
    property bool hasOverride: false
    property bool limitsExpanded: false
    /*! Optional daemon hold map: { remainingS, observedPercent }. */
    property var externalHold: null
    /*! Hides the hold immediately after a cancel click until the next status. */
    property bool externalHoldDismissed: false

    /*! Emitted while dragging (throttled) and once on release. */
    signal requested(int percent)
    signal limitsRequested(int minPct, int maxPct)
    signal clearOverrideRequested()
    signal clearExternalHoldRequested()

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property bool hasValue: percent >= 0
    readonly property bool hasExternalHold: externalHold !== null && externalHold !== undefined
        && Number(externalHold.remainingS) > 0
    readonly property bool showExternalHold: hasExternalHold && !externalHoldDismissed
    readonly property real externalHoldSlotWidth: Math.round(94 * root.theme.scaleFactor)
    readonly property real externalHoldSlotHeight: Math.max(root.theme.gridUnit,
                                                               root.theme.fontSizeSmall + root.theme.spacing)
    readonly property string valueText: hasValue
        ? i18ndc(domain, "Brightness percentage", "%1%", percent)
        : i18ndc(domain, "No value is known yet", "—")

    function holdShortText(remainingS) {
        const seconds = Math.max(0, Number(remainingS) || 0);
        return seconds < 60 ? "<1m" : Math.ceil(seconds / 60) + "m";
    }

    function holdLongText(remainingS) {
        const seconds = Math.max(0, Number(remainingS) || 0);
        return seconds < 60 ? "<1 min" : Math.ceil(seconds / 60) + " min";
    }

    function externalHoldBadgeText(remainingS) {
        return i18ndc(domain, "External brightness hold badge", "external · %1",
                      holdShortText(remainingS));
    }

    function externalHoldTooltipText(remainingS) {
        return i18ndc(domain, "External brightness hold tooltip",
                      "Brightness was changed outside SunReactor; auto resumes in %1",
                      holdLongText(remainingS));
    }

    function cancelExternalHold() {
        externalHoldDismissed = true;
        clearExternalHoldRequested();
    }

    onExternalHoldChanged: {
        if (!hasExternalHold) {
            externalHoldDismissed = false;
        }
    }

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
        spacing: root.theme.compact ? Math.round(Kirigami.Units.largeSpacing * root.theme.scaleFactor) : root.theme.gridUnit

        Kirigami.Icon {
            Layout.alignment: Qt.AlignTop
            Layout.preferredWidth: Math.round(Kirigami.Units.iconSizes.medium * root.theme.scaleFactor)
            Layout.preferredHeight: Layout.preferredWidth
            source: root.iconName
            color: root.theme.fg
            isMask: root.theme.available
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            spacing: root.theme.spacing

            RowLayout {
                Layout.fillWidth: true
                spacing: root.theme.spacing

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.text
                    textFormat: Text.PlainText
                    font.family: root.theme.bodyFont
                    font.pixelSize: root.theme.fontSizeBody
                    color: root.theme.fg
                    elide: Text.ElideRight
                }

                // Keep a fixed-height slot so a hold never changes the row's
                // measured height; its width is reserved to avoid reflow too.
                Item {
                    Layout.preferredWidth: root.externalHoldSlotWidth
                    Layout.minimumWidth: root.externalHoldSlotWidth
                    Layout.maximumWidth: root.externalHoldSlotWidth
                    Layout.preferredHeight: root.externalHoldSlotHeight
                    Layout.minimumHeight: root.externalHoldSlotHeight
                    Layout.maximumHeight: root.externalHoldSlotHeight

                    RowLayout {
                        anchors.fill: parent
                        spacing: root.theme.spacing / 2

                        PlasmaComponents3.Label {
                            Layout.fillWidth: true
                            visible: root.showExternalHold
                            text: root.externalHold !== null && root.externalHold !== undefined
                                ? root.externalHoldBadgeText(root.externalHold.remainingS) : ""
                            textFormat: Text.PlainText
                            font.family: root.theme.digitsFont
                            font.pixelSize: root.theme.fontSizeSmall
                            color: root.theme.warning
                            elide: Text.ElideRight
                        }

                        PlasmaComponents3.ToolButton {
                            id: clearHoldButton

                            Layout.preferredWidth: root.theme.gridUnit
                            Layout.preferredHeight: root.theme.gridUnit
                            visible: root.showExternalHold
                            enabled: visible
                            icon.name: "window-close"
                            display: PlasmaComponents3.AbstractButton.IconOnly
                            text: i18nd(root.domain, "Cancel external brightness hold")
                            onClicked: root.cancelExternalHold()
                            Accessible.name: text
                        }
                    }

                    HoverHandler { id: holdHover }
                    QQC2.ToolTip.visible: holdHover.hovered && root.showExternalHold
                    QQC2.ToolTip.text: root.showExternalHold
                        ? root.externalHoldTooltipText(root.externalHold.remainingS) : ""
                    QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
                }

                // In a manual override, one click returns to the solar curve.
                PlasmaComponents3.Button {
                    visible: root.hasOverride
                    icon.name: "edit-undo"
                    text: i18ndc(root.domain, "Return to automatic solar tracking", "Auto")
                    display: PlasmaComponents3.AbstractButton.TextBesideIcon
                    onClicked: root.clearOverrideRequested()
                }

                PlasmaComponents3.Label {
                    text: root.hint
                    textFormat: Text.PlainText
                    font.family: root.theme.bodyFont
                    font.pixelSize: root.theme.fontSizeSmall
                    color: root.theme.textMuted
                    visible: root.hint.length > 0 && !root.hasOverride
                }

                PlasmaComponents3.ToolButton {
                    visible: root.hasLimits
                    checkable: true
                    checked: root.limitsExpanded
                    icon.name: root.limitsExpanded ? "arrow-up" : "configure"
                    text: i18ndc(root.domain, "Range limits", "%1%–%2%", root.minPct, root.maxPct)
                    display: PlasmaComponents3.AbstractButton.TextBesideIcon
                    font.family: root.theme.digitsFont
                    onClicked: root.limitsExpanded = !root.limitsExpanded
                }

                PlasmaComponents3.Label {
                    text: root.valueText
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                    font.family: root.theme.digitsFont
                    font.pixelSize: root.theme.fontSizeBody
                    color: root.hasValue && (slider.pressed || root.activeFocus) ? root.theme.accent : root.theme.fg
                }
            }

            ThemeSlider {
                id: slider

                theme: root.theme
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

            // Day maximum and night minimum, folded away until asked for.
            GridLayout {
                Layout.fillWidth: true
                visible: root.hasLimits && root.limitsExpanded
                columns: 4
                columnSpacing: root.theme.spacing
                rowSpacing: 0

                Kirigami.Icon {
                    Layout.preferredWidth: Math.round(Kirigami.Units.iconSizes.small * root.theme.scaleFactor)
                    Layout.preferredHeight: Math.round(Kirigami.Units.iconSizes.small * root.theme.scaleFactor)
                    source: "weather-clear-night"
                }

                PlasmaComponents3.Label {
                    text: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                    font.pixelSize: root.theme.fontSizeSmall
                    color: root.theme.textMuted
                }

                ThemeSlider {
                    id: minSlider

                    theme: root.theme
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
                            value = Qt.binding(() => root.minPct);
                        }
                    }
                    Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                }

                PlasmaComponents3.Label {
                    Layout.preferredWidth: root.theme.gridUnit * 2
                    horizontalAlignment: Text.AlignRight
                    text: i18ndc(root.domain, "Percentage", "%1%", Math.round(minSlider.value))
                    font.features: ({ "tnum": 1 })
                    font.family: root.theme.digitsFont
                    font.pixelSize: root.theme.fontSizeSmall
                    color: root.theme.fg
                }

                Kirigami.Icon {
                    Layout.preferredWidth: Math.round(Kirigami.Units.iconSizes.small * root.theme.scaleFactor)
                    Layout.preferredHeight: Math.round(Kirigami.Units.iconSizes.small * root.theme.scaleFactor)
                    source: "weather-clear"
                }

                PlasmaComponents3.Label {
                    text: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                    font.pixelSize: root.theme.fontSizeSmall
                    color: root.theme.textMuted
                }

                ThemeSlider {
                    id: maxSlider

                    theme: root.theme
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
                            value = Qt.binding(() => root.maxPct);
                        }
                    }
                    Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                }

                PlasmaComponents3.Label {
                    Layout.preferredWidth: root.theme.gridUnit * 2
                    horizontalAlignment: Text.AlignRight
                    text: i18ndc(root.domain, "Percentage", "%1%", Math.round(maxSlider.value))
                    font.features: ({ "tnum": 1 })
                    font.family: root.theme.digitsFont
                    font.pixelSize: root.theme.fontSizeSmall
                    color: root.theme.fg
                }
            }
        }
    }
}
