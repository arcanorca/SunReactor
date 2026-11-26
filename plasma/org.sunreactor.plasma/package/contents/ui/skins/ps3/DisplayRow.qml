/*
 * One XMB list item for a display (or the master row): the name with its value
 * on one line, a thin slider under them, and when there is something to say the
 * state and the actions beneath. The item the pointer or the keyboard is on
 * grows a little, gets a soft white halo and brightens; every item has a faint
 * separator below. Drags are throttled, and the thumb follows the daemon again
 * once released (the same contract as components/DisplayItem.qml).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

import "../../components"

Item {
    id: row

    required property XmbStyle style
    property string name: ""
    /*! Percentage to display, or -1 when the daemon has no value for it. */
    property int percent: -1
    /*! Where the automatic curve wants this display now; -1 for none. */
    property real target: -1
    property int minPct: -1
    property int maxPct: -1
    property bool hasLimits: false
    property bool hasOverride: false
    /*! False disables the slider: a display that is off, or not responding. */
    property bool controllable: true
    /*! Short state note, e.g. "Manual" or "Off". */
    property string stateText: ""
    property bool limitsExpanded: false

    readonly property bool hasValue: percent >= 0
    readonly property bool emphasised: hover.hovered || slider.visualFocus || minSlider.visualFocus
        || maxSlider.visualFocus || range.activeFocus || auto.activeFocus
    readonly property string valueText: hasValue
        ? i18ndc("plasma_applet_org.sunreactor.plasma", "Brightness percentage", "%1%", percent)
        : "—"

    /*! The user asked for a brightness (throttled while dragging, once on release). */
    signal requested(int percent)
    signal limitsRequested(int minPct, int maxPct)
    signal limitsToggled()
    signal clearOverrideRequested()

    /*! A text action: Tab reaches it, Space or Enter runs it, a click too. */
    component Link: FocusScope {
        id: link

        property string label: ""
        property color tone: row.style.accentSoft
        signal triggered()

        implicitWidth: text.implicitWidth + row.style.gap2
        implicitHeight: text.implicitHeight + row.style.gap
        activeFocusOnTab: true

        Keys.onReturnPressed: link.triggered()
        Keys.onSpacePressed: link.triggered()

        Text {
            id: text

            anchors.centerIn: parent
            text: link.label
            textFormat: Text.PlainText
            font: row.style.legendFont
            color: link.activeFocus || mouse.containsMouse ? row.style.ink : link.tone
        }

        // Keyboard focus: a hairline under the action.
        Rectangle {
            visible: link.activeFocus
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            height: row.style.line
            color: row.style.ink
        }

        MouseArea {
            id: mouse

            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: {
                link.forceActiveFocus(Qt.MouseFocusReason);
                link.triggered();
            }
        }
    }

    Layout.fillWidth: true
    // The halo reaches one gap past the row on each side: the row keeps that room
    // inside its slot, so the module's clip never cuts the halo.
    Layout.leftMargin: row.style.gap
    Layout.rightMargin: row.style.gap
    implicitWidth: 200
    implicitHeight: content.implicitHeight + style.gap2

    HoverHandler {
        id: hover
    }

    // XMB horizontal spotlight beam behind the emphasised item.
    Image {
        visible: row.emphasised
        property bool auditIgnore: true
        anchors.fill: parent
        anchors.margins: -row.style.gap
        source: Qt.resolvedUrl("assets/ps3_spotlight.png")
        fillMode: Image.Stretch
        smooth: true
    }

    // The faint separator.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: row.style.hairline
        color: row.style.rule
    }

    ColumnLayout {
        id: content

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.topMargin: row.style.gap
        spacing: row.style.gap * 0.5

        // Emphasis: scaled from the left edge, centred vertically.
        transform: Scale {
            origin.x: 0
            origin.y: content.height / 2
            xScale: row.emphasised ? 1.03 : 1
            yScale: row.emphasised ? 1.03 : 1
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: row.style.gap2

            Text {
                Layout.fillWidth: true
                text: row.name
                textFormat: Text.PlainText
                font: row.style.labelFont
                color: row.emphasised ? row.style.ink : row.style.inkSoft
                elide: Text.ElideRight
            }

            // The automatic range: the limits open below.
            Link {
                id: range

                visible: row.hasLimits
                label: row.minPct + "–" + row.maxPct
                tone: row.style.inkSoft
                onTriggered: row.limitsToggled()
            }

            Text {
                text: row.valueText
                textFormat: Text.PlainText
                horizontalAlignment: Text.AlignRight
                font: row.style.valueFont
                color: row.controllable ? row.style.ink : row.style.inkSoft
            }
        }

        XmbSlider {
            id: slider

            Layout.fillWidth: true
            style: row.style
            enabled: row.controllable && row.hasValue
            known: row.hasValue
            target: row.target
            limitMin: row.hasLimits ? row.minPct : -1
            limitMax: row.hasLimits ? row.maxPct : -1
            value: row.hasValue ? row.percent : 0

            // While dragging, ignore status updates: the daemon reports the value
            // it has written, which lags behind the thumb.
            onPressedChanged: {
                if (pressed) {
                    return;
                }
                throttle.release(Math.round(value));
                value = Qt.binding(() => row.hasValue ? row.percent : 0);
            }
            onMoved: pressed ? throttle.schedule(Math.round(value)) : throttle.nudge(Math.round(value))
        }

        // The state and the way back, in the legend size.
        RowLayout {
            Layout.fillWidth: true
            visible: row.stateText.length > 0 || row.hasOverride
            spacing: row.style.gap2

            Text {
                Layout.fillWidth: true
                text: row.stateText
                textFormat: Text.PlainText
                font: row.style.legendFont
                color: row.style.inkSoft
                elide: Text.ElideRight
            }

            Link {
                id: auto

                visible: row.hasOverride
                label: i18ndc("plasma_applet_org.sunreactor.plasma",
                              "Return to automatic solar tracking", "Auto")
                onTriggered: row.clearOverrideRequested()
            }
        }

        // The limits, when the range is open: a minimum and a maximum slider.
        ColumnLayout {
            Layout.fillWidth: true
            visible: row.hasLimits && row.limitsExpanded
            spacing: row.style.gap

            XmbSlider {
                id: minSlider

                Layout.fillWidth: true
                style: row.style
                enabled: row.controllable && row.hasLimits
                from: 0
                to: Math.min(100, Math.round(maxSlider.value))
                value: row.minPct >= 0 ? row.minPct : 0
                onMoved: pressed ? minThrottle.schedule(Math.round(value)) : minThrottle.nudge(Math.round(value))
                onPressedChanged: {
                    if (!pressed) {
                        minThrottle.release(Math.round(value));
                        value = Qt.binding(() => row.minPct >= 0 ? row.minPct : 0);
                    }
                }
            }

            XmbSlider {
                id: maxSlider

                Layout.fillWidth: true
                style: row.style
                enabled: row.controllable && row.hasLimits
                from: Math.max(0, Math.round(minSlider.value))
                to: 100
                value: row.maxPct >= 0 ? row.maxPct : 100
                onMoved: pressed ? maxThrottle.schedule(Math.round(value)) : maxThrottle.nudge(Math.round(value))
                onPressedChanged: {
                    if (!pressed) {
                        maxThrottle.release(Math.round(value));
                        value = Qt.binding(() => row.maxPct >= 0 ? row.maxPct : 100);
                    }
                }
            }
        }
    }

    // DDC/CI writes travel over an I2C bus that does not like packet storms,
    // so a drag sends one request every quarter second, not one per pixel.
    CommitThrottle {
        id: throttle

        reportedValue: row.hasValue ? row.percent : -1
        onCommitted: val => {
            row.requested(val);
            if (!slider.pressed) {
                slider.value = Qt.binding(() => row.hasValue ? row.percent : 0);
            }
        }
    }

    CommitThrottle {
        id: minThrottle

        reportedValue: row.minPct
        onCommitted: val => {
            row.limitsRequested(val, Math.round(maxSlider.value));
            if (!minSlider.pressed) {
                minSlider.value = Qt.binding(() => row.minPct >= 0 ? row.minPct : 0);
            }
        }
    }

    CommitThrottle {
        id: maxThrottle

        reportedValue: row.maxPct
        onCommitted: val => {
            row.limitsRequested(Math.round(minSlider.value), val);
            if (!maxSlider.pressed) {
                maxSlider.value = Qt.binding(() => row.maxPct >= 0 ? row.maxPct : 100);
            }
        }
    }
}
