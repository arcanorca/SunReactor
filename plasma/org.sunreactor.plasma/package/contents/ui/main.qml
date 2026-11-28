/*
 * Sun- and weather-adaptive display brightness, as a Plasma widget.
 *
 * The daemon owns the policy; this widget shows what it is doing, lets the
 * user override it, and stays out of the way otherwise. Loads the native C++
 * plugin through DaemonBridge.qml so a KDE Store install without the native
 * daemon/plugin displays an aesthetic single-shot setup card instead of failing.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami

import "components"
import "Icons.js" as Icons

PlasmoidItem {
    id: root

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    function tr(msg: string): string {
        return i18nd(root.domain, msg); // qmllint disable unqualified
    }

    readonly property bool pluginReady: daemonLoader.status === Loader.Ready && daemonLoader.item !== null
    readonly property var daemon: pluginReady ? daemonLoader.item.client : null // qmllint disable missing-property
    readonly property bool isDesktop: (Plasmoid.formFactor === PlasmaCore.Types.Planar
                                       || Plasmoid.location === PlasmaCore.Types.Floating)
        && (!pluginReady || !daemon.harnessMode // qmllint disable missing-property
            || root.harness.desktopMode === true || root.harness.desktopMode === "true")

    preferredRepresentation: root.isDesktop ? fullRepresentation : null
    Plasmoid.backgroundHints: root.isDesktop
        ? (PlasmaCore.Types.NoBackground | PlasmaCore.Types.ConfigurableBackground)
        : PlasmaCore.Types.DefaultBackground

    // Plasma closes a popup the moment its window loses focus. With KWin's
    // focus-follows-mouse that is the moment the pointer leaves it, so a pointer
    // that overshoots the edge by a pixel closes the popup. Close it ourselves,
    // after a grace period, and only if the window is still inactive and the
    // pointer has not come back.
    hideOnWindowDeactivate: false
    readonly property var popupWindow: root.fullRepresentationItem ? root.fullRepresentationItem.Window.window : null
    readonly property bool menuOpen: root.fullRepresentationItem
        && root.fullRepresentationItem.activePopupMenus > 0 // qmllint disable missing-property

    Timer {
        id: deactivateGrace

        interval: 1000
        onTriggered: {
            const w = root.popupWindow;
            const hovered = root.fullRepresentationItem && root.fullRepresentationItem.popupHovered === true; // qmllint disable missing-property
            if (!root.isDesktop && root.expanded && !(w && w.active) && !hovered && !root.menuOpen) {
                root.expanded = false;
            }
        }
    }
    Connections {
        target: root.popupWindow
        ignoreUnknownSignals: true

        function onActiveChanged(): void {
            if (root.isDesktop) {
                return;
            }
            if (root.popupWindow.active || root.menuOpen) {
                deactivateGrace.stop();
            } else if (root.expanded) {
                deactivateGrace.restart();
            }
        }
    }
    Connections {
        target: root.fullRepresentationItem
        ignoreUnknownSignals: true

        function onPopupHoveredChanged(): void {
            if (root.isDesktop) {
                return;
            }
            if (root.fullRepresentationItem.popupHovered || root.menuOpen) { // qmllint disable missing-property
                deactivateGrace.stop();
            } else if (root.expanded && !(root.popupWindow && root.popupWindow.active)) {
                deactivateGrace.restart();
            }
        }
    }
    onMenuOpenChanged: {
        if (root.isDesktop) {
            return;
        }
        if (root.menuOpen) {
            deactivateGrace.stop();
        } else if (root.expanded
                   && !(root.popupWindow && root.popupWindow.active)
                   && !(root.fullRepresentationItem && root.fullRepresentationItem.popupHovered)) { // qmllint disable missing-property
            deactivateGrace.restart();
        }
    }
    onExpandedChanged: {
        if (!root.expanded) {
            deactivateGrace.stop();
            const item = root.fullRepresentationItem;
            if (item && typeof item.closePopups === "function") { // qmllint disable missing-property
                item.closePopups(); // qmllint disable missing-property
            }
        }
    }

    Loader {
        id: daemonLoader

        source: Qt.resolvedUrl("DaemonBridge.qml")
        onStatusChanged: {
            if (status === Loader.Error) {
                console.info("SunReactor native C++ plugin not found; showing SetupGuide.");
            }
        }
    }

    Binding {
        when: root.pluginReady
        target: root.daemon
        property: "socketPath"
        // Plasma adds the main.xml configuration key dynamically.
        value: Plasmoid.configuration.socketPath // qmllint disable missing-property
    }

    Binding {
        when: root.pluginReady
        target: root.daemon
        property: "popupOpen"
        // Poll faster while the popup is on screen, or continuously when live on desktop.
        value: root.expanded || root.isDesktop
    }

    WeatherText {
        id: weatherText
    }

    // The generic icon of the widget (explorer, tray fallback): the Breeze
    // symbol for the weather the policy is reacting to. The panel draws the
    // sky itself (panel/Sky.qml). Only a paused daemon replaces it, because
    // then the sky is not driving anything.
    Plasmoid.icon: {
        if (!root.pluginReady) {
            return "org.sunreactor.plasma";
        }
        if (!root.daemon.isConnected) { // qmllint disable missing-property
            return Icons.themeIcon("", false);
        }
        if (root.daemon.isSuspended) { // qmllint disable missing-property
            return "media-playback-pause";
        }
        if (root.daemon.hasWeatherReading) { // qmllint disable missing-property
            return Icons.themeIcon(root.daemon.weatherCondition, root.daemon.weatherIsNight); // qmllint disable missing-property
        }
        return Icons.themeIcon("clear", !root.daemon.isDaylight); // qmllint disable missing-property
    }

    // Passive when there is nothing to control: a missing daemon is not an
    // emergency worth pulling the icon out of the system tray for.
    Plasmoid.status: root.pluginReady && root.daemon.isConnected // qmllint disable missing-property
        ? PlasmaCore.Types.ActiveStatus
        : PlasmaCore.Types.PassiveStatus

    switchWidth: root.isDesktop ? 0 : Kirigami.Units.gridUnit * 12
    switchHeight: root.isDesktop ? 0 : Kirigami.Units.gridUnit * 12
    toolTipTextFormat: Text.PlainText

    toolTipMainText: {
        if (!root.pluginReady) {
            return root.tr("SunReactor setup needed");
        }
        if (!root.daemon.isConnected) { // qmllint disable missing-property
            return root.tr("Not connected");
        }

        const monitors = root.daemon.monitors; // qmllint disable missing-property
        if (monitors.length === 1) {
            const only = monitors[0];
            return only.percent >= 0
                ? i18ndc(root.domain, "Placeholder is a percentage",
                         "Screen brightness at %1%", only.percent)
                : root.tr("Screen brightness is not known yet");
        }

        const lines = [];
        for (const monitor of monitors) {
            lines.push(monitor.percent >= 0
                ? i18ndc(root.domain, "Display name, then its brightness percentage",
                         "%1 at %2%", monitor.logicalId, monitor.percent)
                : i18ndc(root.domain, "Placeholder is a display name",
                         "%1 is not known yet", monitor.logicalId));
        }
        return lines.length > 0 ? lines.join("\n") : root.tr("No displays are configured");
    }

    toolTipSubText: {
        if (!root.pluginReady) {
            return root.tr("Open the widget to copy the one-command installer");
        }
        if (!root.daemon.isConnected) { // qmllint disable missing-property
            return root.tr("The background service is not answering");
        }

        const lines = [];
        switch (root.daemon.mode) { // qmllint disable missing-property
        case 1: // SunReactorClient.Paused
            lines.push(root.tr("Paused"));
            break;
        case 2: // SunReactorClient.Manual
            lines.push(root.tr("Set by hand"));
            break;
        case 3: // SunReactorClient.IdleDimmed
            lines.push(root.tr("Dimmed while idle"));
            break;
        default:
            lines.push(root.tr("Following the sun"));
            break;
        }

        if (root.daemon.hasWeatherReading) { // qmllint disable missing-property
            lines.push(weatherText.summary(root.daemon));
        }

        lines.push(root.daemon.isSuspended // qmllint disable missing-property
            ? root.tr("Middle-click to resume")
            : root.tr("Middle-click to pause"));

        return lines.join("\n");
    }

    Plasmoid.contextualActions: [
        PlasmaCore.Action {
            text: root.pluginReady && root.daemon.isSuspended // qmllint disable missing-property
                ? i18ndc(root.domain, "@action:inmenu", "Resume Automatic Brightness")
                : i18ndc(root.domain, "@action:inmenu", "Pause Automatic Brightness")
            icon.name: root.pluginReady && root.daemon.isSuspended ? "media-playback-start" : "media-playback-pause" // qmllint disable missing-property
            enabled: root.pluginReady && root.daemon.isConnected // qmllint disable missing-property
            onTriggered: {
                if (!root.pluginReady) {
                    return;
                }
                if (root.daemon.isSuspended) { // qmllint disable missing-property
                    root.daemon.resume(); // qmllint disable missing-property
                    return;
                }
                // Plasma adds the main.xml configuration key dynamically.
                const minutes = Plasmoid.configuration.defaultSuspendMinutes; // qmllint disable missing-property
                root.daemon.suspend(minutes > 0 ? minutes : 60); // qmllint disable missing-property
            }
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Switch Back to Automatic")
            icon.name: "edit-undo"
            visible: root.pluginReady && root.daemon.isOverrideActive // qmllint disable missing-property
            onTriggered: if (root.pluginReady) root.daemon.clearAllOverrides() // qmllint disable missing-property
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Update Now")
            icon.name: "view-refresh"
            enabled: root.pluginReady && root.daemon.isConnected // qmllint disable missing-property
            onTriggered: if (root.pluginReady) root.daemon.runOnce(true) // qmllint disable missing-property
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Fetch Weather Again")
            icon.name: "weather-many-clouds"
            visible: root.pluginReady && root.daemon.weatherEnabled // qmllint disable missing-property
            onTriggered: if (root.pluginReady) root.daemon.refreshWeather() // qmllint disable missing-property
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Reload Service Configuration")
            icon.name: "document-revert"
            enabled: root.pluginReady && root.daemon.isConnected // qmllint disable missing-property
            priority: PlasmaCore.Action.LowPriority
            onTriggered: if (root.pluginReady) root.daemon.reloadConfig() // qmllint disable missing-property
        },
        PlasmaCore.Action {
            text: i18ndc(root.domain, "@action:inmenu", "Open in Terminal…")
            icon.name: "utilities-terminal"
            visible: root.pluginReady
            priority: PlasmaCore.Action.LowPriority
            onTriggered: if (root.pluginReady) root.daemon.openTerminal() // qmllint disable missing-property
        }
    ]

    function applyHarnessOverrides(): void {
        if (!root.pluginReady || !root.daemon) {
            return;
        }
        const overrides = root.daemon.harnessConfig(); // qmllint disable missing-property
        for (const key of Object.keys(overrides)) {
            if (Plasmoid.configuration[key] !== undefined) {
                Plasmoid.configuration[key] = overrides[key];
            }
        }
    }

    onPluginReadyChanged: applyHarnessOverrides()
    Component.onCompleted: applyHarnessOverrides()

    // Screenshot / Nest harness config (empty unless SUNREACTOR_HARNESS is set).
    readonly property var harness: root.pluginReady && root.daemon ? root.daemon.harnessConfig() : ({}) // qmllint disable missing-property
    readonly property var harnessScript: {
        const steps = [];
        if (!root.harness.open) {
            return steps;
        }
        steps.push({ delay: Number(root.harness.openDelay || 1200), run: () => { root.expanded = true; } });
        const tabs = String(root.harness.tabs || "").split(",").filter(page => page.length > 0);
        for (const page of tabs) {
            steps.push({ delay: 1500, run: () => { root.harnessShowPage(page); } });
        }
        if (root.harness.reopen) {
            steps.push({ delay: 1500, run: () => { root.expanded = false; } });
            steps.push({ delay: 1500, run: () => { root.expanded = true; } });
        }
        return steps;
    }

    property string harnessLast: ""

    Timer {
        id: harnessProbe

        interval: 100
        repeat: true
        running: root.harnessScript.length > 0
        onTriggered: {
            const item = root.fullRepresentationItem;
            const w = item ? item.harnessWindow : null; // qmllint disable missing-property
            const text = w && w.visible ? [w.x, w.y, w.width, w.height].join(" ") : "hidden";
            if (text !== root.harnessLast) {
                root.harnessLast = text;
                console.info("SRWINDOW " + Date.now() + " " + text);
            }
        }
    }

    function harnessShowPage(page: string): void {
        const item = root.fullRepresentationItem;
        if (item && typeof item.showPage === "function") { // qmllint disable missing-property
            item.showPage(page); // qmllint disable missing-property
        }
    }

    Timer {
        id: harnessTimer

        property int next: 0

        interval: root.harnessScript.length > 0 ? root.harnessScript[0].delay : 1000
        running: root.harnessScript.length > 0
        onTriggered: {
            root.harnessScript[next].run();
            next += 1;
            if (next < root.harnessScript.length) {
                interval = root.harnessScript[next].delay;
                restart();
            }
        }
    }

    compactRepresentation: Item {
        id: compactShell

        readonly property Item loadedCompact: compactLoader.item as Item

        Layout.minimumWidth: loadedCompact ? loadedCompact.Layout.minimumWidth : Kirigami.Units.iconSizes.medium
        Layout.preferredWidth: loadedCompact ? loadedCompact.Layout.preferredWidth : Kirigami.Units.iconSizes.medium
        Layout.maximumWidth: loadedCompact ? loadedCompact.Layout.maximumWidth : Number.POSITIVE_INFINITY
        Layout.minimumHeight: loadedCompact ? loadedCompact.Layout.minimumHeight : Kirigami.Units.iconSizes.medium
        Layout.preferredHeight: loadedCompact ? loadedCompact.Layout.preferredHeight : Kirigami.Units.iconSizes.medium
        Layout.maximumHeight: loadedCompact ? loadedCompact.Layout.maximumHeight : Number.POSITIVE_INFINITY

        Loader {
            id: compactLoader

            anchors.fill: parent
            active: root.pluginReady

            function loadCompact(): void {
                if (active && root.daemon) {
                    setSource(Qt.resolvedUrl("CompactRepresentation.qml"), {
                        plasmoidItem: root,
                        client: root.daemon,
                    });
                } else {
                    source = "";
                }
            }

            onActiveChanged: loadCompact()
            Component.onCompleted: loadCompact()
        }

        MouseArea {
            anchors.fill: parent
            visible: !root.pluginReady
            acceptedButtons: Qt.LeftButton
            cursorShape: Qt.PointingHandCursor
            onClicked: root.expanded = !root.expanded

            Kirigami.Icon {
                anchors.centerIn: parent
                width: Math.min(parent.width, parent.height, Kirigami.Units.iconSizes.medium)
                height: width
                source: "org.sunreactor.plasma"
            }
        }
    }

    Component {
        id: popupRep

        Item {
            id: popupShell

            readonly property bool collapseMarginsHint: true
            readonly property Item loadedPopup: popupLoader.item as Item
            readonly property var client: loadedPopup ? loadedPopup.client : root.daemon // qmllint disable missing-property
            readonly property var theme: loadedPopup ? loadedPopup.theme : null // qmllint disable missing-property
            readonly property var harness: loadedPopup ? loadedPopup.harness : root.harness // qmllint disable missing-property
            readonly property Item contentItem: loadedPopup ? loadedPopup.contentItem : null // qmllint disable missing-property
            readonly property Item footer: loadedPopup ? loadedPopup.footer : null // qmllint disable missing-property
            readonly property string shownPage: loadedPopup ? loadedPopup.shownPage : "" // qmllint disable missing-property
            readonly property string currentPage: loadedPopup ? loadedPopup.currentPage : "" // qmllint disable missing-property
            readonly property bool tabsShown: loadedPopup ? loadedPopup.tabsShown : false // qmllint disable missing-property
            readonly property real popupHeight: loadedPopup ? loadedPopup.popupHeight : height // qmllint disable missing-property
            readonly property real popupMaxHeight: loadedPopup ? loadedPopup.popupMaxHeight : height // qmllint disable missing-property
            readonly property bool popupHovered: loadedPopup ? loadedPopup.popupHovered : false // qmllint disable missing-property
            readonly property int activePopupMenus: loadedPopup ? loadedPopup.activePopupMenus : 0 // qmllint disable missing-property
            readonly property var harnessWindow: loadedPopup ? loadedPopup.harnessWindow : null // qmllint disable missing-property

            function showPage(page: string): void {
                if (popupLoader.item && typeof popupLoader.item.showPage === "function") { // qmllint disable missing-property
                    popupLoader.item.showPage(page); // qmllint disable missing-property
                }
            }

            function closePopups(): void {
                if (popupLoader.item && typeof popupLoader.item.closePopups === "function") { // qmllint disable missing-property
                    popupLoader.item.closePopups(); // qmllint disable missing-property
                }
            }

            Layout.minimumWidth: loadedPopup ? loadedPopup.Layout.minimumWidth : Kirigami.Units.gridUnit * 16
            Layout.preferredWidth: loadedPopup ? loadedPopup.Layout.preferredWidth : Kirigami.Units.gridUnit * 19
            Layout.maximumWidth: loadedPopup ? loadedPopup.Layout.maximumWidth : Kirigami.Units.gridUnit * 40
            Layout.minimumHeight: loadedPopup ? loadedPopup.Layout.minimumHeight : Kirigami.Units.gridUnit * 14
            Layout.preferredHeight: loadedPopup ? loadedPopup.Layout.preferredHeight : Kirigami.Units.gridUnit * 20
            Layout.maximumHeight: loadedPopup ? loadedPopup.Layout.maximumHeight : Kirigami.Units.gridUnit * 34

            Loader {
                id: popupLoader

                anchors.fill: parent
                active: root.pluginReady

                function loadPopup(): void {
                    if (active && root.daemon) {
                        setSource(Qt.resolvedUrl("FullRepresentation.qml"), {
                            client: root.daemon,
                        });
                    } else {
                        source = "";
                    }
                }

                onActiveChanged: loadPopup()
                Component.onCompleted: loadPopup()
            }
        }
    }

    Component {
        id: desktopRep

        Item {
            id: desktopShell

            readonly property Item loadedDesktop: desktopLoader.item as Item
            readonly property var client: loadedDesktop ? loadedDesktop.client : root.daemon // qmllint disable missing-property
            readonly property var theme: loadedDesktop ? loadedDesktop.theme : null // qmllint disable missing-property
            readonly property var harness: loadedDesktop ? loadedDesktop.harness : root.harness // qmllint disable missing-property
            readonly property bool popupHovered: loadedDesktop ? loadedDesktop.popupHovered : false // qmllint disable missing-property
            readonly property int activePopupMenus: loadedDesktop ? loadedDesktop.activePopupMenus : 0 // qmllint disable missing-property
            readonly property var harnessWindow: loadedDesktop ? loadedDesktop.harnessWindow : null // qmllint disable missing-property

            function showPage(page: string): void {
                if (desktopLoader.item && typeof desktopLoader.item.showPage === "function") { // qmllint disable missing-property
                    desktopLoader.item.showPage(page); // qmllint disable missing-property
                }
            }

            function closePopups(): void {
                if (desktopLoader.item && typeof desktopLoader.item.closePopups === "function") { // qmllint disable missing-property
                    desktopLoader.item.closePopups(); // qmllint disable missing-property
                }
            }

            Layout.minimumWidth: loadedDesktop ? loadedDesktop.Layout.minimumWidth : Kirigami.Units.gridUnit * 10
            Layout.preferredWidth: loadedDesktop ? loadedDesktop.Layout.preferredWidth : Kirigami.Units.gridUnit * 19
            Layout.maximumWidth: Number.POSITIVE_INFINITY
            Layout.minimumHeight: loadedDesktop ? loadedDesktop.Layout.minimumHeight : Kirigami.Units.gridUnit * 10
            Layout.preferredHeight: loadedDesktop ? loadedDesktop.Layout.preferredHeight : Kirigami.Units.gridUnit * 20
            Layout.maximumHeight: Number.POSITIVE_INFINITY

            Loader {
                id: desktopLoader

                anchors.fill: parent
                active: root.pluginReady

                function loadDesktop(): void {
                    if (active && root.daemon) {
                        setSource(Qt.resolvedUrl("DesktopRepresentation.qml"), {
                            client: root.daemon,
                        });
                    } else {
                        source = "";
                    }
                }

                onActiveChanged: loadDesktop()
                Component.onCompleted: loadDesktop()
            }
        }
    }

    Component {
        id: missingPluginRep

        PlasmaExtras.Representation {
            Layout.minimumWidth: Kirigami.Units.gridUnit * 18
            Layout.preferredWidth: Kirigami.Units.gridUnit * 21
            Layout.maximumWidth: Kirigami.Units.gridUnit * 28
            Layout.minimumHeight: guide.implicitHeight + 2 * Kirigami.Units.largeSpacing
            Layout.preferredHeight: guide.implicitHeight + 2 * Kirigami.Units.largeSpacing

            contentItem: SetupGuide {
                id: guide

                anchors.margins: Kirigami.Units.largeSpacing
                pluginMissing: true
                domain: root.domain
            }
        }
    }

    fullRepresentation: !root.pluginReady ? missingPluginRep : (root.isDesktop ? desktopRep : popupRep)
}
