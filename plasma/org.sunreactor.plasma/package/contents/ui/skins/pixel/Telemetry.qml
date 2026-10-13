/*
 * Pixel Telemetry: the status line as the hardware's own top bar.
 *   dmg    bezel print: state LED, then the printed rules of the DMG bezel
 *   c64    READY line with the blinking block cursor
 *   mac    System 1 menu bar (sun where the Apple menu was, clock right)
 *   casio  watch face: seven-segment clock with ghost segments, indicators
 *   amiga  Workbench screen title bar with the depth gadget
 *   dots / neon  state dot, state word, clock
 * Module contract: modules/Modules.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

import "../../engine"
import "../../modules"
import "PixelFormat.js" as PixelFormat
import "Sprites.js" as Sprites

Item {
    id: root

    required property Theme theme
    required property SunReactorClient client
    required property ModuleSettings settings

    readonly property string domain: settings.domain
    readonly property bool available: client.isConnected

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

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
    readonly property string tempText: client.hasTemperature
        ? i18ndc(domain, "Temperature in degrees Celsius, compact", "%1°C", Math.round(client.temperatureC)) : ""
    readonly property string rightText: [clock.hm + clock.meridiem, tempText].filter(s => s.length > 0).join(" ")
    readonly property bool big: pstyle.kind === "casio"
    readonly property color barText: pstyle.kind === "amiga" ? theme.bg : (pstyle.kind === "mac" ? pstyle.ink : theme.fg)

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: [stateText, rightText].join(" · ")

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: root.big ? 24 : 13
        regionName: "telemetry"

        readonly property int w: artWidth
        readonly property int mid: Math.floor((artHeight - 7) / 2)

        // Bar background.
        Rectangle {
            width: surface.w
            height: surface.artHeight
            antialiasing: false
            color: pstyle.kind === "mac" ? pstyle.paper : (pstyle.kind === "amiga" ? root.theme.fg : pstyle.desk)
        }
        Rectangle {
            visible: pstyle.kind === "mac" || pstyle.kind === "amiga"
            y: surface.artHeight - 1
            width: surface.w
            height: 1
            antialiasing: false
            color: pstyle.kind === "mac" ? pstyle.ink : root.theme.bg
        }

        // State LED (mac: the sun symbol in the Apple menu's place).
        PixelSprite {
            visible: pstyle.kind === "mac"
            x: 6
            y: surface.mid
            source: Sprites.symbolAtlas()
            frame: Sprites.symbol("sun")
            frameWidth: 8
            frameHeight: 8
            colors: pstyle.spriteColors
        }
        Rectangle {
            id: led
            visible: pstyle.kind !== "mac"
            x: 3
            y: root.big ? 4 : surface.mid + 2
            width: 3
            height: 3
            antialiasing: false
            color: root.theme.stateColor(root.client.mode)
        }

        PixelLabel {
            id: stateLabel
            theme: root.theme
            x: pstyle.kind === "mac" ? 18 : 9
            y: root.big ? 3 : surface.mid
            text: root.stateText
            face: pstyle.bodyFace
            color: pstyle.kind === "amiga" ? root.theme.bg
                : (pstyle.kind === "mac" ? pstyle.ink : root.theme.stateColor(root.client.mode))
        }

        // C64: blinking block cursor after the state word.
        Rectangle {
            id: cursor
            visible: pstyle.kind === "c64" && blink.on
            x: stateLabel.x + stateLabel.artWidth + 2
            y: stateLabel.y
            width: 7
            height: 8
            antialiasing: false
            color: root.theme.fg
        }
        Timer {
            id: blink
            property bool on: true
            interval: 530
            repeat: true
            running: pstyle.kind === "c64" && surface.visible && root.client.popupOpen
            onTriggered: on = !on
            onRunningChanged: on = true
        }

        // DMG: the printed rules of the bezel between the two groups.
        Item {
            visible: pstyle.kind === "dmg"
            x: stateLabel.x + stateLabel.artWidth + 4
            y: surface.mid + 1
            width: Math.max(0, rightGroup.x - x - 4)
            Rectangle { y: 0; width: parent.width; height: 1; color: root.theme.accent; antialiasing: false }
            Rectangle { y: 3; width: parent.width; height: 1; color: root.theme.borderInactive; antialiasing: false }
        }

        // Casio: day-of-week indicator under the state, like the watch's AM/PM flags.
        PixelLabel {
            visible: root.big
            theme: root.theme
            x: 3
            y: 14
            face: "tiny"
            capitals: true
            color: root.theme.accent
            text: root.client.nowEpochS > 0
                ? Qt.locale().dayName(new Date(root.client.nowEpochS * 1000).getDay(), Locale.ShortFormat)
                  + " " + new Date(root.client.nowEpochS * 1000).getDate()
                : ""
        }

        // Right group.
        Item {
            id: rightGroup
            x: surface.w - width - (pstyle.kind === "amiga" ? 22 : 3)
            y: 0
            width: root.big ? bigClock.width + 3 + meridiem.artWidth + (temp.visible ? 4 + temp.artWidth : 0)
                            : rightLabel.artWidth
            height: surface.artHeight

            PixelLabel {
                id: rightLabel
                visible: !root.big
                theme: root.theme
                y: surface.mid
                text: root.rightText
                face: pstyle.bodyFace
                color: root.barText
            }

            PixelDigits {
                id: bigClock
                visible: root.big
                theme: root.theme
                style: pstyle
                showGhost: true
                y: 3
                size: 2
                text: root.clock.hm
            }
            PixelLabel {
                id: meridiem
                visible: root.big
                theme: root.theme
                x: bigClock.width + 3
                y: 3
                face: "tiny"
                capitals: true
                color: root.theme.accent
                text: root.clock.meridiem
            }
            PixelDigits {
                id: temp
                visible: root.big && root.client.hasTemperature
                theme: root.theme
                style: pstyle
                x: bigClock.width + 3 + meridiem.artWidth + 4
                y: 3 + bigClock.height - height
                size: 1
                text: Math.round(root.client.temperatureC) + "°"
            }
        }

        // Amiga: depth gadget at the right end of the screen bar.
        Item {
            visible: pstyle.kind === "amiga"
            x: surface.w - 19
            y: 2
            Rectangle { x: 0; y: 0; width: 9; height: 6; color: root.theme.bg; antialiasing: false }
            Rectangle { x: 1; y: 1; width: 7; height: 4; color: root.theme.fg; antialiasing: false }
            Rectangle { x: 5; y: 3; width: 9; height: 6; color: root.theme.bg; antialiasing: false }
            Rectangle { x: 6; y: 4; width: 7; height: 4; color: root.theme.secondaryAccent; antialiasing: false }
        }
    }
}
