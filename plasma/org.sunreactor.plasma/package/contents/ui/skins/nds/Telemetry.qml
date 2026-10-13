/*
 * Nintendo DS status bar: a thin strip above the menu, as the console's own top
 * bar - wifi bars (lit when the daemon answers), the daemon state word, then
 * the battery lamp (lit by the state) and the clock on the right. Module
 * contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "../pixel"
import "../pixel/PixelFormat.js" as PixelFormat
import "Nds.js" as Nds

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected

    readonly property string stateText: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return i18ndc(domain, "Daemon state in the status line", "OFFLINE");
        case SunReactorClient.Paused:
            return i18ndc(domain, "Daemon state in the status line", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(domain, "Daemon state in the status line", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(domain, "Daemon state in the status line", "IDLE");
        default:
            return i18ndc(domain, "Daemon state in the status line", "LIVE");
        }
    }
    readonly property var clock: PixelFormat.timeParts(client.nowEpochS)
    /*! Battery cells: the daemon's state, from none (offline) to all three (live). */
    readonly property int lamp: {
        switch (client.mode) {
        case SunReactorClient.Offline:
            return 0;
        case SunReactorClient.Paused:
        case SunReactorClient.Manual:
            return 2;
        case SunReactorClient.IdleDimmed:
            return 1;
        default:
            return 3;
        }
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: stateText + " " + clock.hm + clock.meridiem

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: Nds.TELEMETRY_H
        regionName: "telemetry"

        Rectangle {
            width: surface.artWidth
            height: surface.artHeight
            color: root.theme.bg
            antialiasing: false
        }
        Rectangle {
            y: surface.artHeight - 1
            width: surface.artWidth
            height: 1
            color: root.theme.borderInactive
            antialiasing: false
        }

        // Wifi bars, bottom aligned: three rising bars.
        Repeater {
            model: 3
            delegate: Rectangle {
                required property int index

                x: 3 + 3 * index
                y: surface.artHeight - 2 - (3 + 2 * index)
                width: 2
                height: 3 + 2 * index
                color: root.available ? root.theme.accent : root.theme.borderInactive
                antialiasing: false
            }
        }

        PixelLabel {
            id: stateLabel

            theme: root.theme
            x: 14
            y: 1
            face: "small"
            capitals: true
            text: root.stateText
            color: root.theme.stateColor(root.client.mode)
        }

        PixelLabel {
            id: clockLabel

            theme: root.theme
            x: surface.artWidth - 4 - artWidth
            y: 1
            face: "small"
            capitals: true
            text: root.clock.hm + root.clock.meridiem
            color: root.theme.fg
        }

        Repeater {
            model: Nds.battery([], clockLabel.x - 15, 2, root.lamp,
                               { frame: root.theme.fg, lit: root.theme.stateColor(root.client.mode),
                                 faint: root.theme.borderInactive })
            delegate: Rectangle {
                required property var modelData

                x: modelData.x
                y: modelData.y
                width: modelData.w
                height: modelData.h
                color: modelData.c
                antialiasing: false
            }
        }
    }
}
