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
import org.sunreactor.plasma

import "../../engine"
import "../../modules"

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property NdsStyle style: NdsStyle {
        theme: root.theme
    }
    readonly property string domain: settings.domain
    readonly property var monitors: client.isConnected ? client.monitors : []
    /*! The automation controls sit under the panels on this screen when the theme lists them. */
    readonly property bool hostsModes: root.theme.modules.indexOf("automation") >= 0
    readonly property bool available: client.isConnected && (monitors.length > 0 || hostsModes)
    readonly property var rows: {
        const out = [];
        if (monitors.length > 1) {
            out.push({ master: true, logicalId: i18nd(domain, "All displays"), percent: client.globalPercent,
                       hasOverride: client.isOverrideActive, enabled: true, unreachable: false });
        }
        for (const m of monitors) {
            out.push(m);
        }
        return out;
    }

    readonly property real gu: style.gu
    readonly property real pad: Math.round(gu * 0.35)
    readonly property real inset: Math.round(gu * 0.6)
    readonly property real panelH: Math.round(gu * 3.0)
    readonly property real gap: Math.round(gu * 0.25)
    readonly property real panelsH: rows.length * panelH + Math.max(0, rows.length - 1) * gap
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

        Repeater {
            model: root.rows

            delegate: DisplayPanel {
                x: root.inset
                y: root.pad + index * (root.panelH + root.gap)
                width: screen.glassWidth - 2 * root.inset
                height: root.panelH
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

        readonly property bool master: modelData.master === true
        readonly property bool unavailable: modelData.unreachable === true
            || modelData.topology === "temporarily_unavailable"
        readonly property bool controllable: modelData.enabled !== false && !unavailable
        readonly property int percent: modelData.percent !== undefined ? modelData.percent : -1
        readonly property int minPct: modelData.minPct !== undefined ? modelData.minPct : -1
        readonly property int maxPct: modelData.maxPct !== undefined ? modelData.maxPct : -1
        readonly property string hint: {
            if (modelData.enabled === false) {
                return i18ndc(root.domain, "This display is excluded from automation", "Off");
            }
            if (unavailable) {
                return i18ndc(root.domain, "This display is not answering, compact", "N/A");
            }
            return "";
        }
        readonly property bool overridden: modelData.hasOverride === true || knob.dragValue >= 0
        readonly property string name: master ? i18nd(root.domain, "All displays") : String(modelData.logicalId)

        style: root.style
        kind: controllable ? "panel" : "quiet"
        outlineColor: controllable ? root.style.blue : root.style.line

        Accessible.role: Accessible.Grouping
        Accessible.name: name

        Text {
            x: Math.round(root.gu * 0.8)
            y: Math.round(root.gu * 0.42)
            width: valueText.x - x - Math.round(root.gu * 0.5)
            elide: Text.ElideRight
            text: panel.name
            font.family: root.style.family
            font.weight: Font.Bold
            font.pixelSize: Math.round(root.gu * 0.9)
            color: panel.controllable ? root.style.ink : root.style.muted
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
            color: panel.hint.length > 0 ? root.style.muted
                : (panel.overridden ? root.style.blue : root.style.ink)
        }

        KnobSlider {
            id: knob

            x: Math.round(root.gu * 0.5)
            height: Math.max(28, Math.round(root.gu * 1.6))
            y: Math.round(panel.height - height - root.gu * 0.2)
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
    }

    /*! A round knob on a track, 0..100. Holding it writes the value at a steady
        rate and once more on release; arrows step it by 5. */
    component KnobSlider: Item {
        id: slider

        property int value: -1
        property int minMark: -1
        property int maxMark: -1
        property bool editable: true
        property string accessibleName: ""
        /*! The value under the finger while it is held, -1 when it is not. */
        property real dragValue: -1
        readonly property real shown: dragValue >= 0 ? dragValue : value
        readonly property real knobSize: Math.round(root.gu * 1.5)
        readonly property real travelX0: knobSize / 2
        readonly property real travelX1: width - knobSize / 2
        readonly property real knobX: travelX0 + (travelX1 - travelX0) * Math.max(0, Math.min(100, shown)) / 100
        property int lastSent: -1

        signal committed(int percent)

        function fromX(x) {
            const f = (x - travelX0) / Math.max(1, travelX1 - travelX0);
            return Math.round(Math.max(0, Math.min(1, f)) * 100);
        }

        function markX(pct) {
            return travelX0 + (travelX1 - travelX0) * pct / 100;
        }

        function send() {
            const v = Math.round(dragValue);
            if (v >= 0 && v !== lastSent) {
                lastSent = v;
                committed(v);
            }
        }

        function step(delta) {
            if (!editable || shown < 0) {
                return;
            }
            const v = Math.max(0, Math.min(100, Math.round(shown) + delta));
            dragValue = v;
            lastSent = v;
            committed(v);
        }

        onValueChanged: {
            if (!mouse.pressed) {
                dragValue = -1;
            }
        }

        activeFocusOnTab: editable
        Keys.onLeftPressed: slider.step(-5)
        Keys.onRightPressed: slider.step(5)

        Accessible.role: Accessible.Slider
        Accessible.name: accessibleName
        Accessible.description: shown >= 0 ? Math.round(shown) + "%" : ""

        // The track, its fill, the automatic range's ends, and the knob.
        Rectangle {
            x: slider.travelX0
            y: Math.round(slider.height / 2 - height / 2)
            width: slider.travelX1 - slider.travelX0
            height: Math.round(root.gu * 0.42)
            radius: height / 2
            color: slider.editable ? root.style.track : root.style.quietBottom
        }
        Rectangle {
            x: slider.travelX0
            y: Math.round(slider.height / 2 - height / 2)
            width: Math.max(0, slider.knobX - slider.travelX0)
            height: Math.round(root.gu * 0.42)
            radius: height / 2
            color: slider.editable ? root.style.blue : root.style.muted
            visible: slider.shown >= 0
        }
        Rectangle {
            visible: slider.minMark >= 0
            x: Math.round(slider.markX(slider.minMark)) - 1
            y: Math.round(slider.height / 2 - height / 2)
            width: 2
            height: Math.round(root.gu * 0.8)
            color: root.style.muted
        }
        Rectangle {
            visible: slider.maxMark >= 0
            x: Math.round(slider.markX(slider.maxMark)) - 1
            y: Math.round(slider.height / 2 - height / 2)
            width: 2
            height: Math.round(root.gu * 0.8)
            color: root.style.muted
        }
        Rectangle {
            x: Math.round(slider.knobX - slider.knobSize / 2)
            y: Math.round(slider.height / 2 - slider.knobSize / 2)
            width: slider.knobSize
            height: slider.knobSize
            radius: width / 2
            border.width: root.style.outline
            border.color: slider.editable ? root.style.blue : root.style.line
            gradient: Gradient {
                GradientStop { position: 0; color: root.style.white }
                GradientStop { position: 1; color: root.style.panelBottom }
            }
            visible: slider.shown >= 0
        }

        // Holding the knob's hit area sets the value; the whole row is the target.
        MouseArea {
            id: mouse

            anchors.fill: parent
            enabled: slider.editable
            cursorShape: Qt.PointingHandCursor
            onPressed: event => {
                slider.forceActiveFocus(Qt.MouseFocusReason);
                slider.lastSent = -1;
                slider.dragValue = slider.fromX(event.x);
                slider.send();
            }
            onPositionChanged: event => {
                if (mouse.pressed) {
                    slider.dragValue = slider.fromX(event.x);
                }
            }
            onReleased: event => {
                slider.dragValue = slider.fromX(event.x);
                slider.send();
            }
            onCanceled: slider.send()
        }

        // Steady writes while the knob is held (a repeat, not an animation).
        Timer {
            interval: 150
            repeat: true
            running: mouse.pressed
            onTriggered: slider.send()
        }
    }
}
