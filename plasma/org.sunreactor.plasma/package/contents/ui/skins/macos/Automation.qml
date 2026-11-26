/*
 * Automation, macos: the mode as a native segmented control (Automatic | Manual |
 * Paused, a chevron on Paused opens the pause durations), then what the daemon is
 * doing and until when, in 11 px secondary text. The rules are in
 * modules/ModeControl.qml; this file only draws them.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../../components"

ColumnLayout {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }
    readonly property ModeControl control: ModeControl {
        client: root.client
        settings: root.settings
    }
    readonly property bool available: client.isConnected || client.statusStale
    /*! The control's choices as segments: { id, text }. */
    readonly property var segments: control.choices.map(choice => ({ id: choice.id, text: choice.label }))

    spacing: style.gap

    Accessible.role: Accessible.Grouping
    Accessible.name: i18ndc(control.domain, "Accessible name of the automation mode switch", "Automation mode")

    Segmented {
        Layout.fillWidth: true
        style: root.style
        segments: root.segments
        current: root.control.choice
        arrowId: "paused"

        onPicked: id => root.control.pick(id, root)
        onArrowPicked: (id, anchorItem) => durations.openBelow(anchorItem, root)
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Math.max(2, Math.round(root.style.gap * 0.35))

        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.control.primaryText
            textFormat: Text.PlainText
            font: root.style.captionMedium
            color: root.control.hinting ? root.style.warning : root.style.ink
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.control.scheduleText
            textFormat: Text.PlainText
            font: root.style.caption
            color: root.style.inkSecondary
            wrapMode: Text.WordWrap
            maximumLineCount: 2
            elide: Text.ElideRight
        }

        Text {
            Layout.fillWidth: true
            visible: text.length > 0
            text: root.control.note
            textFormat: Text.PlainText
            font: root.style.caption
            color: root.style.inkSecondary
        }
    }

    ModeDurationMenu {
        id: durations

        control: root.control
    }
}
