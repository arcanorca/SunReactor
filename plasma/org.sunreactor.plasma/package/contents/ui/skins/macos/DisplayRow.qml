/*
 * One display in the brightness list: its name above a tall pill slider, the
 * value on the slider's right, and the small gray controls. Auto gives a value
 * set by hand back to the curve; Range opens the day-maximum and night-minimum
 * sliders underneath. A hairline separates it from the next row.
 *
 *   Name                                     [Auto] [Range 15–60 ▾]
 *   ████████████████████████████░░░░░░░░░░░░  39%
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property MacStyle style
    /*! The translation domain of the widget (ModuleSettings.domain). */
    required property string domain
    property string name: ""
    /*! The value the row shows, -1 while unknown. */
    property int percent: -1
    property int minPct: 15
    property int maxPct: 60
    property bool hasLimits: false
    property bool hasOverride: false
    property bool controllable: true
    /*! Short state word in the name row ("Off", "No link", "Manual"); empty for none. */
    property string stateText: ""
    property color stateTone: style.inkSecondary
    property bool divided: false
    property bool limitsExpanded: false

    signal requested(int percent)
    signal limitsRequested(int minVal, int maxVal)
    signal clearOverrideRequested()

    readonly property bool hasValue: percent >= 0

    implicitWidth: body.implicitWidth + 2 * style.padRow
    implicitHeight: body.implicitHeight + 2 * style.snap(7)

    ColumnLayout {
        id: body

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: root.style.padRow
        anchors.rightMargin: root.style.padRow
        anchors.topMargin: root.style.snap(7)
        spacing: root.style.snap(3)

        // --- name row -----------------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.gap

            Text {
                Layout.fillWidth: true
                text: root.name
                textFormat: Text.PlainText
                font: root.style.caption
                color: root.style.inkSecondary
                elide: Text.ElideMiddle
                opacity: root.controllable ? 1 : 0.6
            }

            Text {
                visible: root.stateText.length > 0
                text: root.stateText
                textFormat: Text.PlainText
                font: root.style.captionMedium
                color: root.stateTone
            }

            Chip {
                visible: root.hasOverride
                style: root.style
                text: i18ndc(root.domain,
                             "Return to automatic solar tracking", "Auto")
                onClicked: root.clearOverrideRequested()
            }

            Chip {
                visible: root.hasLimits
                style: root.style
                chevron: true
                open: root.limitsExpanded
                text: i18ndc(root.domain,
                             "Range button: the automatic minimum and maximum", "Range %1–%2",
                             root.minPct, root.maxPct)
                onClicked: root.limitsExpanded = !root.limitsExpanded
            }
        }

        // --- slider and value -------------------------------------------------
        RowLayout {
            Layout.fillWidth: true
            spacing: root.style.snap(10)

            DisplaySlider {
                id: fader

                Layout.fillWidth: true
                style: root.style
                percent: root.percent
                enabled: root.controllable && root.hasValue
                Accessible.name: root.name
                Accessible.description: root.hasValue
                    ? i18ndc(root.domain, "Brightness percentage", "%1%", root.percent)
                    : i18ndc(root.domain, "No value is known yet", "—")

                onRequested: p => root.requested(p)
            }

            Text {
                Layout.preferredWidth: root.style.snap(38)
                horizontalAlignment: Text.AlignRight
                text: root.hasValue ? root.percent + "%" : "—"
                textFormat: Text.PlainText
                font: root.style.value
                color: root.hasValue ? root.style.ink : root.style.inkSecondary
            }
        }

        // --- automatic range: day maximum and night minimum -----------------
        ColumnLayout {
            Layout.fillWidth: true
            visible: root.hasLimits && root.limitsExpanded
            spacing: root.style.snap(6)

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.snap(10)

                Text {
                    Layout.preferredWidth: root.style.snap(58)
                    text: i18ndc(root.domain, "Day maximum boundary", "Day max")
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.inkSecondary
                }
                DisplaySlider {
                    id: dayMax

                    Layout.fillWidth: true
                    style: root.style
                    trackHeight: root.style.snap(18)
                    showSun: false
                    percent: root.maxPct
                    from: Math.round(nightMin.value)
                    onRequested: p => root.limitsRequested(Math.round(nightMin.value), p)
                    Accessible.name: i18ndc(root.domain, "Day maximum boundary", "Day maximum")
                }
                Text {
                    Layout.preferredWidth: root.style.snap(30)
                    horizontalAlignment: Text.AlignRight
                    text: Math.round(dayMax.value) + "%"
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.ink
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: root.style.snap(10)

                Text {
                    Layout.preferredWidth: root.style.snap(58)
                    text: i18ndc(root.domain, "Night minimum boundary", "Night min")
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.inkSecondary
                }
                DisplaySlider {
                    id: nightMin

                    Layout.fillWidth: true
                    style: root.style
                    trackHeight: root.style.snap(18)
                    showSun: false
                    percent: root.minPct
                    to: Math.round(dayMax.value)
                    onRequested: p => root.limitsRequested(p, Math.round(dayMax.value))
                    Accessible.name: i18ndc(root.domain, "Night minimum boundary", "Night minimum")
                }
                Text {
                    Layout.preferredWidth: root.style.snap(30)
                    horizontalAlignment: Text.AlignRight
                    text: Math.round(nightMin.value) + "%"
                    textFormat: Text.PlainText
                    font: root.style.caption
                    color: root.style.ink
                }
            }
        }
    }

    // The separator is indented to the text, like an inset list's.
    Rectangle {
        visible: root.divided
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.leftMargin: root.style.padRow
        height: root.style.hairline
        color: root.style.separator
    }
}
