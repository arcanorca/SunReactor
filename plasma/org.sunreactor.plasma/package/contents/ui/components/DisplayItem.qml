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

PlasmaComponents3.ItemDelegate {
    id: root

    /*! Percentage to display, or -1 when the daemon has no value for it. */
    required property int percent
    property string iconName: "video-display-brightness"
    /*! Short state note shown before the value, e.g. "Manual". */
    property string hint: ""
    /*! False disables the slider: a display that is off, or not responding. */
    property bool controllable: true

    /*! Emitted while dragging (debounced) and once on release. */
    signal requested(int percent)

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
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                PlasmaComponents3.Label {
                    Layout.fillWidth: true
                    text: root.text
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                }

                PlasmaExtras.DescriptiveLabel {
                    text: root.hint
                    textFormat: Text.PlainText
                    visible: root.hint.length > 0
                }

                PlasmaComponents3.Label {
                    text: root.valueText
                    textFormat: Text.PlainText
                    font.features: ({ "tnum": 1 })
                }
            }

            PlasmaComponents3.Slider {
                id: slider

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
        }
    }
}
