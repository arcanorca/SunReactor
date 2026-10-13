/*
 * Pixel Automation: mode, why, and what happens next ("16:20 SET 50% → 23%"),
 * with the way back to automatic when the user has taken over. Hidden while
 * simply following the sun with nothing next to report.
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

    readonly property var nextMilestone: {
        const curve = client.dayCurve;
        if (!curve || !curve.monitors || curve.monitors.length === 0) {
            return null;
        }
        for (const milestone of curve.monitors[0].milestones) {
            if (milestone.t > client.nowEpochS) {
                return milestone;
            }
        }
        return null;
    }
    readonly property string nextText: nextMilestone
        ? (nextMilestone.pct >= 0
            ? i18ndc(domain, "Time, then the daemon's milestone name, then the brightness it targets",
                     "%1 · %2 → %3%", PixelFormat.time(nextMilestone.t), nextMilestone.label, nextMilestone.pct)
            : i18ndc(domain, "Time, then the daemon's milestone name", "%1 · %2",
                     PixelFormat.time(nextMilestone.t), nextMilestone.label))
        : ""

    readonly property bool available: client.isConnected
        && (client.mode !== SunReactorClient.Automatic || nextText.length > 0)

    readonly property string symbol: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return "pause";
        case SunReactorClient.Manual:
            return "hand";
        case SunReactorClient.IdleDimmed:
            return "moon";
        default:
            return "clock";
        }
    }
    readonly property string modeTitle: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return i18nd(domain, "Paused");
        case SunReactorClient.Manual:
            return i18nd(domain, "Set by hand");
        case SunReactorClient.IdleDimmed:
            return i18nd(domain, "Dimmed while idle");
        default:
            return client.hasWeatherReading
                ? i18nd(domain, "Following the sun and the weather")
                : i18nd(domain, "Following the sun");
        }
    }
    readonly property string modeDetail: {
        switch (client.mode) {
        case SunReactorClient.Paused:
            return client.suspendUntilEpochS > 0
                ? i18ndc(domain, "Placeholder is a time of day", "Until %1", PixelFormat.time(client.suspendUntilEpochS))
                : i18nd(domain, "Until you resume it");
        case SunReactorClient.Manual:
            return client.overrideUntilEpochS > 0
                ? i18ndc(domain, "Placeholder is a time of day", "Until %1", PixelFormat.time(client.overrideUntilEpochS))
                : i18nd(domain, "Until you switch back to automatic");
        case SunReactorClient.IdleDimmed:
            return i18nd(domain, "Restored as soon as you come back");
        default:
            return nextText.length > 0
                ? i18ndc(domain, "Placeholder is the next step of the brightness curve", "Next: %1", nextText)
                : "";
        }
    }

    PixelStyle {
        id: pstyle
        theme: root.theme
    }

    implicitHeight: surface.implicitHeight

    Accessible.role: Accessible.StaticText
    Accessible.name: modeTitle
    Accessible.description: modeDetail

    PixelSurface {
        id: surface

        width: parent.width
        grid: root.theme.grid
        fillsWidth: true
        artHeight: panel.implicitHeight
        regionName: "automation"

        PixelPanel {
            id: panel

            theme: root.theme
            style: pstyle
            width: surface.artWidth
            title: i18ndc(root.domain, "Title of the automation panel", "Mode")
            contentHeight: Math.max(10, titleText.height + (detail.visible ? 2 + detail.height : 0))
                + (button.visible ? button.height + 4 : 0)

            PixelSprite {
                id: icon
                y: 0
                source: Sprites.symbolAtlas()
                frame: Sprites.symbol(root.symbol)
                frameWidth: 8
                frameHeight: 8
                colors: {
                    const c = pstyle.spriteColors.slice();
                    c[4] = root.client.mode === SunReactorClient.Automatic ? pstyle.ink : pstyle.hot;
                    c[2] = pstyle.ink;
                    c[3] = pstyle.ink;
                    c[8] = "transparent";
                    return c;
                }
            }
            PixelLabel {
                id: titleText
                theme: root.theme
                x: 12
                text: root.modeTitle
                face: pstyle.bodyFace
                color: root.client.mode === SunReactorClient.Automatic ? pstyle.ink : pstyle.hot
                maximumArtWidth: panel.contentWidth - 12
                maximumLines: 2
            }
            PixelLabel {
                id: detail
                visible: text.length > 0
                theme: root.theme
                x: 12
                y: titleText.height + 2
                text: root.modeDetail
                face: pstyle.bodyFace
                color: pstyle.dim
                stipple: pstyle.stippleDim
                maximumArtWidth: panel.contentWidth - 12
                maximumLines: 2
            }
            PixelButton {
                id: button
                visible: root.client.isOverrideActive
                theme: root.theme
                style: pstyle
                x: panel.contentWidth - width
                y: panel.contentHeight - height
                text: i18ndc(root.domain, "@action:button Return to automatic brightness", "Automatic")
                onClicked: root.client.clearAllOverrides()
            }
        }
    }
}
