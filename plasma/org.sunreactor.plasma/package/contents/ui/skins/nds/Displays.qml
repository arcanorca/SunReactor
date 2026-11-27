/*
 * Nintendo DS touch screen, displays: one big panel button per display - the name
 * on the left, the brightness in big rounded digits on the right, and a round
 * knob slider along the bottom of the panel that takes touch, drag and keys.
 * "All displays" comes first once there is more than one. Unknown stays "--"
 * with an inert knob. A drag writes overrides at a steady rate and once more on
 * release; scrolling never writes. Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../../components"
import "../../engine"
import "../../modules"
import "../../modules/Monitors.js" as Monitors

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property var monitors: Monitors.hasStatus(client) ? client.monitors : []
    /*! The automation controls sit under the panels on this screen when the theme lists them. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: Monitors.hasStatus(client) && (monitors.length > 0 || hostsModes)
    readonly property var rows: Monitors.rowsWithMaster(
        monitors, client.globalPercent, client.isOverrideActive, settings.allDisplaysLabel)
    property var expandedLimits: ({})

    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.35)
    readonly property real inset: Math.round(gu * 0.6)
    readonly property real panelH: Math.round(gu * 3.0)
    readonly property real limitsH: Math.round(gu * 3.6)
    readonly property real gap: Math.round(gu * 0.25)
    readonly property real panelsH: panelsCol.implicitHeight
    /*! The tray (ModeRows) below the panels, when this screen hosts it. */
    readonly property real trayGap: Math.round(gu * 0.45)
    readonly property real glassH: 2 * pad + panelsH + (hostsModes ? trayGap + modes.trayH : 0)

    implicitHeight: screen.implicitHeight

    Accessible.role: Accessible.Grouping
    Accessible.name: i18nd(domain, "Displays")

    NdsScreen {
        id: screen

        width: parent.width
        theme: root.theme
        style: root.style
        wallpaper: "touch"
        glassHeight: root.glassH

        Column {
            id: panelsCol

            x: root.inset
            y: root.pad
            width: screen.glassWidth - 2 * root.inset
            spacing: root.gap

            Repeater {
                model: root.rows

                delegate: DisplayPanel {
                    width: panelsCol.width
                    height: expanded && !master ? root.panelH + root.limitsH : root.panelH
                }
            }
        }

        // The automation controls, under the panels, with the daemon's sentence in a bubble.
        ModeRows {
            id: modes

            visible: root.hostsModes
            x: root.inset
            y: root.pad + root.panelsH + root.trayGap
            width: screen.glassWidth - 2 * root.inset
            theme: root.theme
            client: root.client
            settings: root.settings
            style: root.style
        }
    }

    /*! One display: a panel button with its name, its value and its knob. */
    component DisplayPanel: NdsPanel {
        id: panel

        required property var modelData
        required property int index
        readonly property bool expanded: !master && Monitors.isExpanded(root.expandedLimits, modelData.logicalId)

        readonly property bool master: modelData.master === true
        readonly property bool unavailable: Monitors.unavailable(modelData)
        readonly property bool controllable: root.client.isConnected && modelData.enabled !== false && !unavailable
        readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
        readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
        readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
        readonly property bool hasLimits: !master && Monitors.hasValidLimits(modelData)
        readonly property string notice: master ? "" : Monitors.monitorNotice(
            modelData, root.settings.contestedNotice, root.settings.deferredNotice)
        readonly property string tagNotice: master ? "" : Monitors.monitorTag(
            modelData, root.settings.pausedTag, root.settings.queuedTag)
        readonly property string hint: {
            if (modelData.enabled === false) {
                return i18ndc(root.domain, "This display is excluded from automation", "Off");
            }
            if (unavailable) {
                return i18ndc(root.domain, "This display is not answering, compact", "N/A");
            }
            if (tagNotice.length > 0) {
                return tagNotice;
            }
            return "";
        }
        readonly property bool overridden: modelData.hasOverride === true || knob.dragValue >= 0
        readonly property string name: master ? root.settings.allDisplaysLabel : String(modelData.logicalId)

        style: root.style
        kind: controllable ? "panel" : "quiet"
        outlineColor: controllable ? root.style.blue : root.style.line

        Accessible.role: Accessible.Grouping
        Accessible.name: name

        Text {
            x: Math.round(root.gu * 0.8)
            y: Math.round(root.gu * 0.42)
            width: (rangePill.visible ? rangePill.x : valueText.x) - x - Math.round(root.gu * 0.4)
            elide: Text.ElideRight
            text: panel.name
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.9)
            color: panel.controllable ? root.style.ink : root.style.muted
        }
        Rectangle {
            id: rangePill

            visible: panel.hasLimits
            x: valueText.x - width - Math.round(root.gu * 0.4)
            y: Math.round(root.gu * 0.34)
            width: rangeLabel.implicitWidth + Math.round(root.gu * 0.6)
            height: Math.round(root.gu * 1.05)
            radius: 0
            color: panel.expanded ? root.style.blue : root.style.headerNeutral
            border.width: 1
            border.color: root.style.frameBlack

            Text {
                id: rangeLabel

                anchors.centerIn: parent
                text: panel.minPct + "–" + panel.maxPct
                font.family: root.style.family
                font.weight: Font.Bold
                font.pixelSize: Math.round(root.gu * 0.65)
                color: panel.expanded ? root.style.barInk : root.style.ink
            }
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.expandedLimits = Monitors.toggleExpanded(root.expandedLimits, panel.modelData.logicalId)
            }
        }
        Text {
            id: valueText

            x: panel.width - Math.round(root.gu * 0.8) - implicitWidth
            y: Math.round(root.gu * 0.28)
            text: panel.hint.length > 0 ? panel.hint
                : (knob.shown >= 0 ? Math.round(knob.shown) + "%" : "--")
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * (panel.hint.length > 0 ? 0.85 : 1.2))
            color: panel.tagNotice.length > 0 ? root.style.accentInk
                : (panel.hint.length > 0 ? root.style.muted
                : (panel.overridden ? root.style.accentInk : root.style.ink))

            HoverHandler {
                id: valueHover
            }
            QQC2.ToolTip.visible: valueHover.hovered && panel.notice.length > 0
            QQC2.ToolTip.text: panel.notice
            QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay
        }

        KnobSlider {
            id: knob

            x: Math.round(root.gu * 0.5)
            height: Math.max(28, Math.round(root.gu * 1.6))
            y: Math.round(root.panelH - height - root.gu * 0.2)
            width: panel.width - Math.round(root.gu * 1.0)
            value: panel.percent
            minMark: panel.master ? -1 : panel.minPct
            maxMark: panel.master ? -1 : panel.maxPct
            editable: panel.controllable
            accessibleName: panel.name
            onCommitted: v => panel.master
                ? root.client.setGlobalOverride(v, root.settings.overrideMinutes)
                : root.client.setMonitorOverride(panel.modelData.logicalId, v, root.settings.overrideMinutes)
        }

        Column {
            id: limitsCol

            visible: panel.expanded && panel.hasLimits
            x: Math.round(root.gu * 0.5)
            y: root.panelH
            width: panel.width - Math.round(root.gu * 1.0)
            spacing: Math.round(root.gu * 0.15)

            Repeater {
                model: [
                    { label: i18ndc(root.domain, "Night minimum boundary, compact", "Min"), which: 0 },
                    { label: i18ndc(root.domain, "Day maximum boundary, compact", "Max"), which: 1 },
                ]

                delegate: Item {
                    id: limitRow

                    required property var modelData
                    readonly property int current: modelData.which === 0 ? panel.minPct : panel.maxPct

                    width: limitsCol.width
                    height: Math.round(root.gu * 1.65)

                    Text {
                        id: limitTag

                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.round(root.gu * 2.2)
                        text: limitRow.modelData.label
                        font.family: root.style.family
                        font.weight: Font.Bold
                        font.pixelSize: Math.round(root.gu * 0.68)
                        color: root.style.muted
                    }
                    KnobSlider {
                        id: limitKnob

                        x: limitTag.width + Math.round(root.gu * 0.2)
                        width: parent.width - x - limitVal.width - Math.round(root.gu * 0.2)
                        height: parent.height
                        from: limitRow.modelData.which === 0 ? 0 : panel.minPct
                        to: limitRow.modelData.which === 0 ? panel.maxPct : 100
                        value: limitRow.current
                        editable: root.client.isConnected && panel.hasLimits
                        accessibleName: limitRow.modelData.which === 0
                            ? i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                            : i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                        onCommitted: v => limitRow.modelData.which === 0
                            ? root.client.setMonitorLimits(panel.modelData.logicalId,
                                                           Math.min(v, panel.maxPct), panel.maxPct)
                            : root.client.setMonitorLimits(panel.modelData.logicalId,
                                                           panel.minPct, Math.max(v, panel.minPct))
                    }
                    Text {
                        id: limitVal

                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.round(root.gu * 2.1)
                        horizontalAlignment: Text.AlignRight
                        text: Math.round(limitKnob.shown) + "%"
                        font.family: root.style.family
                        font.weight: Font.Bold
                        font.pixelSize: Math.round(root.gu * 0.7)
                        color: root.style.ink
                    }
                }
            }
        }
    }

    /*! A round knob on a track, 0..100. Holding it writes the value at a steady
        rate and once more on release; arrows step it by 5. */
    component KnobSlider: Item {
        id: slider

        property int from: 0
        property int to: 100
        property int value: -1
        property int minMark: -1
        property int maxMark: -1
        property bool editable: true
        property string accessibleName: ""
        /*! The value under the finger while it is held, -1 when it is not. */
        property real dragValue: -1
        readonly property real shown: dragValue >= 0 ? dragValue
            : (throttle.pending ? throttle.pendingValue : value)
        readonly property real knobW: Math.round(root.gu * 0.85)
        readonly property real knobH: Math.round(root.gu * 1.15)
        readonly property real travelX0: knobW / 2
        readonly property real travelX1: width - knobW / 2
        readonly property real knobX: travelX0 + (travelX1 - travelX0) * Math.max(0, Math.min(100, shown)) / 100

        signal committed(int percent)

        function fromX(x) {
            const f = (x - travelX0) / Math.max(1, travelX1 - travelX0);
            const raw = Math.round(Math.max(0, Math.min(1, f)) * 100);
            return Math.max(slider.from, Math.min(slider.to, raw));
        }

        function markX(pct) {
            return travelX0 + (travelX1 - travelX0) * pct / 100;
        }

        function step(delta) {
            if (!editable || shown < 0) {
                return;
            }
            const base = throttle.pending ? throttle.pendingValue : Math.round(shown);
            const v = Math.max(slider.from, Math.min(slider.to, base + delta));
            dragValue = v;
            throttle.nudge(v);
        }

        onValueChanged: {
            if (!mouse.pressed && !throttle.pending) {
                dragValue = -1;
            }
        }

        CommitThrottle {
            id: throttle

            reportedValue: slider.value
            onCommitted: val => slider.committed(val)
        }

        activeFocusOnTab: editable
        Keys.onLeftPressed: slider.step(-5)
        Keys.onRightPressed: slider.step(5)
        Keys.onDownPressed: slider.step(-1)
        Keys.onUpPressed: slider.step(1)
        Keys.onPressed: event => {
            if (event.key === Qt.Key_PageDown) {
                slider.step(-10);
                event.accepted = true;
            } else if (event.key === Qt.Key_PageUp) {
                slider.step(10);
                event.accepted = true;
            } else if (event.key === Qt.Key_Home) {
                slider.step(slider.from - Math.round(slider.shown));
                event.accepted = true;
            } else if (event.key === Qt.Key_End) {
                slider.step(slider.to - Math.round(slider.shown));
                event.accepted = true;
            }
        }

        Accessible.role: Accessible.Slider
        Accessible.name: accessibleName
        Accessible.description: shown >= 0 ? Math.round(shown) + "%" : ""

        // The crisp pixel track, its fill, the automatic range's ends, and the pixel thumb.
        Rectangle {
            x: slider.travelX0
            y: Math.round(slider.height / 2 - height / 2)
            width: slider.travelX1 - slider.travelX0
            height: Math.round(root.gu * 0.44)
            radius: 0
            color: slider.editable ? root.style.track : root.style.quietBottom
            border.width: 1
            border.color: root.style.frameBlack
        }
        Rectangle {
            x: slider.travelX0 + 1
            y: Math.round(slider.height / 2 - height / 2)
            width: Math.max(0, slider.knobX - slider.travelX0 - 1)
            height: Math.max(2, Math.round(root.gu * 0.44) - 2)
            radius: 0
            color: slider.editable ? root.style.blue : root.style.muted
            visible: slider.shown >= 0
        }
        Rectangle {
            visible: slider.minMark >= 0
            x: Math.round(slider.markX(slider.minMark)) - 1
            y: Math.round(slider.height / 2 - height / 2)
            width: 2
            height: Math.round(root.gu * 0.8)
            color: root.style.frameBlack
        }
        Rectangle {
            visible: slider.maxMark >= 0
            x: Math.round(slider.markX(slider.maxMark)) - 1
            y: Math.round(slider.height / 2 - height / 2)
            width: 2
            height: Math.round(root.gu * 0.8)
            color: root.style.frameBlack
        }
        // Crisp DS slider handle: clean beveled tab, no oversized sprite.
        Rectangle {
            id: knobCap

            x: Math.round(slider.knobX - slider.knobW / 2)
            y: Math.round(slider.height / 2 - slider.knobH / 2)
            width: slider.knobW
            height: slider.knobH
            radius: 2
            border.width: root.style.outline
            border.color: slider.activeFocus ? root.style.ink : (slider.editable ? root.style.accentInk : root.style.line)
            gradient: Gradient {
                GradientStop { position: 0; color: root.style.white }
                GradientStop { position: 1; color: root.style.panelBottom }
            }
            visible: slider.shown >= 0

            Rectangle {
                x: 2
                y: 2
                width: parent.width - 4
                height: 1
                color: root.style.white
            }
        }

        // Holding the knob's hit area sets the value; the whole row is the target.
        MouseArea {
            id: mouse

            anchors.fill: parent
            enabled: slider.editable
            cursorShape: enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
            onPressed: event => {
                slider.forceActiveFocus(Qt.MouseFocusReason);
                slider.dragValue = slider.fromX(event.x);
                throttle.schedule(slider.dragValue);
            }
            onPositionChanged: event => {
                if (mouse.pressed) {
                    slider.dragValue = slider.fromX(event.x);
                    throttle.schedule(slider.dragValue);
                }
            }
            onReleased: event => {
                slider.dragValue = slider.fromX(event.x);
                throttle.release(slider.dragValue);
                if (!throttle.pending) {
                    slider.dragValue = -1;
                }
            }
            onCanceled: {
                throttle.release(slider.dragValue >= 0 ? slider.dragValue : undefined);
                if (!throttle.pending) {
                    slider.dragValue = -1;
                }
            }
        }
    }
}
