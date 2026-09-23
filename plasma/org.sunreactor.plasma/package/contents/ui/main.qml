/*
 * Sun- and weather-adaptive display brightness, as a Plasma widget.
 *
 * The daemon owns the policy; this widget shows what it is doing, lets the
 * user override it, and stays out of the way otherwise.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "components"
import "Icons.js" as Icons

PlasmoidItem {
    id: root

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    SunReactorClient {
        id: daemon

        socketPath: Plasmoid.configuration.socketPath
        // Poll faster only while the popup is on screen.
        popupOpen: root.expanded
    }

    WeatherText {
        id: weatherText
    }

    // The panel shows the sky itself: the weather the policy is reacting to,
    // as pixel art. Only a paused daemon replaces it, because then the sky is
    // not driving anything.
    Plasmoid.icon: {
        if (!daemon.isConnected) {
            return Icons.art("unknown");
        }
        if (daemon.isSuspended) {
            return "media-playback-pause";
        }
        if (daemon.hasWeatherReading) {
            return Icons.conditionArt(daemon.weatherCondition, daemon.weatherIsNight);
        }
        return Icons.skyArt(!daemon.isDaylight);
    }

    // Passive when there is nothing to control: a missing daemon is not an
    // emergency worth pulling the icon out of the system tray for.
    Plasmoid.status: daemon.isConnected ? PlasmaCore.Types.ActiveStatus : PlasmaCore.Types.PassiveStatus

    switchWidth: Kirigami.Units.gridUnit * 12
    switchHeight: Kirigami.Units.gridUnit * 12
    toolTipTextFormat: Text.PlainText

    toolTipMainText: {
        if (!daemon.isConnected) {
            return i18nd(root.domain, "Not connected");
        }

        const monitors = daemon.monitors;
        if (monitors.length === 1) {
            const only = monitors[0];
            return only.percent >= 0
                ? i18ndc(root.domain, "Placeholder is a percentage",
                         "Screen brightness at %1%", only.percent)
                : i18nd(root.domain, "Screen brightness is not known yet");
        }

        const lines = [];
        for (const monitor of monitors) {
            lines.push(monitor.percent >= 0
                ? i18ndc(root.domain, "Display name, then its brightness percentage",
                         "%1 at %2%", monitor.logicalId, monitor.percent)
                : i18ndc(root.domain, "Placeholder is a display name",
                         "%1 is not known yet", monitor.logicalId));
        }
        return lines.length > 0 ? lines.join("\n") : i18nd(root.domain, "No displays are configured");
    }

    toolTipSubText: {
        if (!daemon.isConnected) {
            return i18nd(root.domain, "The background service is not answering");
        }

        const lines = [];
        switch (daemon.mode) {
        case SunReactorClient.Paused:
            lines.push(i18nd(root.domain, "Paused"));
            break;
        case SunReactorClient.Manual:
            lines.push(i18nd(root.domain, "Set by hand"));
            break;
        case SunReactorClient.IdleDimmed:
            lines.push(i18nd(root.domain, "Dimmed while idle"));
            break;
        default:
            lines.push(i18nd(root.domain, "Following the sun"));
            break;
        }

        if (daemon.hasWeatherReading) {
            lines.push(weatherText.summary(daemon));
        }

        lines.push(daemon.isSuspended
            ? i18nd(root.domain, "Middle-click to resume")
            : i18nd(root.domain, "Middle-click to pause"));

        return lines.join("\n");
    }

    Plasmoid.contextualActions: [
        PlasmaCore.Action {
            text: daemon.isSuspended
                ? i18ndc(root.domain, "@action:inmenu", "Resume Automatic Brightness")
                : i18ndc(root.domain, "@action:inmenu", "Pause Automatic Brightness")
            icon.name: daemon.isSuspended ? "media-playback-start" : "media-playback-pause"
            enabled: daemon.isConnected
            onTriggered: {
                if (daemon.isSuspended) {
                    daemon.resume();
                    return;
                }
                const minutes = Plasmoid.configuration.defaultSuspendMinutes;
                daemon.suspend(minutes > 0 ? minutes : 60);
            }
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Switch Back to Automatic")
            icon.name: "edit-undo"
            visible: daemon.isOverrideActive
            onTriggered: daemon.clearAllOverrides()
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Update Now")
            icon.name: "view-refresh"
            enabled: daemon.isConnected
            onTriggered: daemon.runOnce(true)
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Fetch Weather Again")
            icon.name: "weather-many-clouds"
            visible: daemon.weatherEnabled
            onTriggered: daemon.refreshWeather()
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Reload Service Configuration")
            icon.name: "document-revert"
            enabled: daemon.isConnected
            priority: PlasmaCore.Action.LowPriority
            onTriggered: daemon.reloadConfig()
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Open in Terminal…")
            icon.name: "utilities-terminal"
            priority: PlasmaCore.Action.LowPriority
            onTriggered: daemon.openTerminal()
        }
    ]

    Timer {
        interval: 1500
        running: true
        onTriggered: root.expanded = true
    }

    compactRepresentation: CompactRepresentation {
        plasmoidItem: root
        client: daemon
    }

    fullRepresentation: FullRepresentation {
        client: daemon
    }
}
