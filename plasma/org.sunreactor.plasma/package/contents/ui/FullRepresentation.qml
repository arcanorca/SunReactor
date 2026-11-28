/*
 * The popup: an instrument, composed of modules in the order the theme lists
 * them (theme.widget.modules; see modules/Modules.js for the contract).
 *
 * Structure follows Plasma's own brightness applet - a Representation holding
 * a scrollable column of flat rows, with a footer for the actions. There is no
 * title: the popup does not repeat the name of the widget. When a daemon theme
 * is in effect the content sits on the theme's background.
 *
 * Telemetry is the status line, on top and hidden by default (the config's
 * hiddenModules). While it is hidden, a status lamp at the end of the tab row
 * shows the daemon state. When the theme lists modules for both tab pages
 * (Modules.PAGES), a tab bar under it switches between the pages; each page
 * scrolls on its own and the footer stays put. The popup is as tall as the page
 * on screen needs (see popupHeight).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.core as PlasmaCore
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "components"
import "engine"
import "Format.js" as Format
import "modules"
import "modules/Modules.js" as Modules

PlasmaExtras.Representation {
    id: root

    required property SunReactorClient client

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property Theme theme: engine.theme

    PlasmoidEngine {
        id: engine

        client: root.client
        xboxDarkModeOverride: Plasmoid.configuration.xboxDarkMode || root.harnessXboxDark // qmllint disable missing-property
        nothingLightModeOverride: Plasmoid.configuration.nothingLightMode || root.harnessNothingLight // qmllint disable missing-property
        macosLightModeOverride: Plasmoid.configuration.macosLightMode || root.harnessMacosLight // qmllint disable missing-property
        braunLightModeOverride: Plasmoid.configuration.braunLightMode || root.harnessBraunLight // qmllint disable missing-property
    }

    readonly property bool harnessXboxDark: {
        const cfg = client.harnessConfig();
        return !!(cfg && (cfg.xboxDark === true || cfg.xboxDark === "true"));
    }
    readonly property bool harnessNothingLight: {
        const cfg = client.harnessConfig();
        return !!(cfg && (cfg.nothingLight === true || cfg.nothingLight === "true"));
    }
    readonly property bool harnessMacosLight: {
        const cfg = client.harnessConfig();
        return !!(cfg && (cfg.macosLight === true || cfg.macosLight === "true"));
    }
    readonly property bool harnessBraunLight: {
        const cfg = client.harnessConfig();
        return !!(cfg && (cfg.braunLight === true || cfg.braunLight === "true"));
    }

    ModuleSettings {
        id: moduleSettings

        // Plasma adds the main.xml configuration keys dynamically.
        overrideMinutes: Plasmoid.configuration.overrideDurationMinutes // qmllint disable missing-property
        defaultSuspendMinutes: Plasmoid.configuration.defaultSuspendMinutes // qmllint disable missing-property
        showSolarElevation: Plasmoid.configuration.showSolarElevation // qmllint disable missing-property
    }

    /*! True when hosted directly inside DesktopRepresentation on the Plasma desktop. */
    property bool desktopEmbedded: false

    function closePopups(): void {
        themeButton.closePicker();
    }

    /*! The pointer is over the popup (main.qml keeps the popup open while it is). */
    readonly property bool popupHovered: popupHover.hovered
    /*! Active floating menus or sub-popups inside the popup. main.qml checks this
        so opening a menu (e.g. pause duration) never collapses the popup. */
    property int activePopupMenus: 0
    HoverHandler {
        id: popupHover
    }

    /*! The preferred width of the popup: 19 scaled grid units. */
    readonly property real popupWidth: root.theme.gridUnit * 19
    Layout.minimumWidth: root.theme.gridUnit * 16
    Layout.preferredWidth: root.popupWidth
    Layout.maximumWidth: root.theme.gridUnit * 40
    // The height is the page on screen plus the chrome around it: see popupHeight.
    Layout.minimumHeight: root.popupMinHeight
    Layout.preferredHeight: root.popupHeight
    Layout.maximumHeight: root.popupMaxHeight

    // --- height ----------------------------------------------------------------
    // The popup is as tall as what is on screen: the status line and tab row,
    // the shown page, the footer and the popup margins. Pages are measured from
    // their columns (PagePanel.columnHeight), never from a ScrollView, so the
    // popup height cannot feed back into the measure. A page taller than
    // popupMaxHeight scrolls.

    /*! The smallest popup: 14 grid units. */
    readonly property real popupMinHeight: root.theme.gridUnit * 14
    /*! The largest popup: 75% of the screen's available height, at most 34 grid
        units (34 alone until the screen is known). */
    readonly property real popupMaxHeight: Math.max(root.popupMinHeight, Math.min(
        root.theme.gridUnit * 34,
        Screen.desktopAvailableHeight > 0 ? Screen.desktopAvailableHeight * 0.75 : root.theme.gridUnit * 34))
    /*! The width the terminal skins' grid spans (Theme.gridContentWidth): the page
        column less one grid unit, the width of a page's scrollbar. A page that
        scrolls keeps this width, so its rows and the popup height do not feed
        back into each other. */
    Binding {
        target: root.theme
        property: "gridContentWidth"
        value: pages.width - 2 * root.edgeInset - root.theme.gridUnit
    }

    /*! Status line, tab row and the gap above the pages. */
    readonly property real headerHeight: headerColumn.implicitHeight + pages.Layout.topMargin
    /*! The shown page's column. */
    readonly property real pageHeight: pages.implicitHeight
    /*! The footer and the popup's own top and bottom margins. */
    readonly property real chromeHeight: (popupFooter.visible ? popupFooter.implicitHeight : 0)
                                         + root.topPadding + root.bottomPadding
    readonly property real popupHeight: Math.min(root.popupMaxHeight,
        Math.max(root.popupMinHeight, root.headerHeight + root.pageHeight + root.chromeHeight))

    /*! Plasma can keep a popup size stored for this applet and then ignore the
        layout hints, so the window stays at the stored height while the page
        changes. The window is fitted to the page here instead: the frame around
        the content (the window's height minus the root's) is kept, and at the
        bottom edge the window moves up so that its bottom stays on the panel.
        Only a shown, laid-out window whose height differs is set, so nothing is
        set twice. */
    function fitWindow(): void {
        if (root.desktopEmbedded) {
            return;
        }
        const w = root.Window.window;
        if (!w || !w.visible || root.height <= 0 || w.height < root.height) {
            return;
        }
        if (root.width > 0 && w.width >= root.width) {
            const wantedW = Math.round(root.popupWidth) + (w.width - Math.round(root.width));
            if (wantedW !== w.width) {
                w.width = wantedW;
            }
        }
        const wanted = Math.round(root.popupHeight) + (w.height - Math.round(root.height));
        if (wanted === w.height) {
            return;
        }
        if (Plasmoid.location === PlasmaCore.Types.BottomEdge) {
            w.y = w.y - (wanted - w.height);
        }
        w.height = wanted;
    }
    onPopupWidthChanged: {
        if (!root.desktopEmbedded) {
            Qt.callLater(root.fitWindow);
        }
    }
    onPopupHeightChanged: {
        if (!root.desktopEmbedded) {
            Qt.callLater(root.fitWindow);
        }
    }
    /*! After the window shows, the first layout of a page can settle a little later (a
        curve that arrives late): fit again a few times, 120 ms apart, then stop. */
    Timer {
        id: showFit

        property int tries: 0

        interval: 120
        repeat: true
        onTriggered: {
            root.fitWindow();
            tries += 1;
            if (tries >= 8) {
                stop();
            }
        }
    }
    Connections {
        target: root.Window.window
        ignoreUnknownSignals: true

        function onVisibleChanged(): void {
            if (root.Window.window && root.Window.window.visible) {
                if (!root.desktopEmbedded) {
                    showFit.tries = 0;
                    showFit.restart();
                }
            } else {
                root.closePopups();
            }
            if (!root.desktopEmbedded) {
                Qt.callLater(root.fitWindow);
            }
        }
    }

    collapseMarginsHint: true
    // collapseMarginsHint would extend the content into the Plasma frame's
    // margins (negative paddings). This widget paints its own background over
    // the popup's full width and keeps every padding at zero, so no content
    // sits at a negative offset from the popup's edges.
    leftPadding: 0
    rightPadding: 0
    topPadding: 0
    bottomPadding: 0

    /*! The inset of the content from each side of the popup, so no glyph or
        focus ring of any skin touches the first or last device pixel column. */
    readonly property real edgeInset: Math.round(Kirigami.Units.smallSpacing * root.theme.scaleFactor)

    component ThemeSlot: Loader {
        required property string wanted

        function load(): void {
            if (wanted.length > 0) {
                const props = wanted.indexOf("ps3/Waves.qml") >= 0
                    ? { theme: root.theme, harness: root.harness }
                    : { theme: root.theme };
                setSource(Qt.resolvedUrl(wanted), props);
            } else {
                source = "";
            }
        }

        onWantedChanged: load()
        Component.onCompleted: load()
    }

    /*! A skin's popup backdrop (Modules.BACKDROP): a Gradient the Frame fills with, or nothing. */
    ThemeSlot {
        id: backdropSlot

        wanted: Modules.backdropSource(root.theme.skinIdentity.available ? root.theme.skinIdentity.id : "")
    }

    background: Frame {
        theme: root.theme
        visible: root.theme.available
        pixel: false
        fillColor: root.theme.bg
        fillGradient: backdropSlot.item
        color: root.theme.line
        style: root.theme.border === "none" ? "none" : "line"

        /*! A skin's overlay (Modules.OVERLAY): an Item over the fill, under the content. */
        ThemeSlot {
            id: overlaySlot

            anchors.fill: parent
            wanted: Modules.overlaySource(root.theme.skinIdentity.available ? root.theme.skinIdentity.id : "")
        }
    }

    // --- tab pages -------------------------------------------------------------

    /*! Screenshot harness only (tools/shot.sh --config "tab=weather"): empty otherwise. */
    readonly property string harnessPage: String(client.harnessConfig().tab || "")
    /*! Nest harness only (tools/nest): the popup's own window, which main.qml logs. */
    readonly property var harnessWindow: root.Window.window
    /*! The page the popup starts on each time it opens. */
    readonly property string startPage: Modules.PAGES.indexOf(harnessPage) >= 0 ? harnessPage : Modules.PAGES[0]
    /*! The page the user picked. Kept while the popup is open; in memory only. */
    property string currentPage: startPage
    // A page change can move the tab row and the popup window: re-hang the theme picker.
    onCurrentPageChanged: themeButton.relayout()
    onTabsShownChanged: themeButton.relayout()

    /*! The status line: modules that stay above the tab bar, in the theme's order. */
    readonly property var topModules: root.theme.modules.filter(id => Modules.ABOVE_TABS.indexOf(id) >= 0)
    /*! Whether the status line is on screen (its module is loaded and available). */
    readonly property bool statusLineShown: {
        for (let i = 0; i < topRepeater.count; ++i) {
            const slot = topRepeater.itemAt(i);
            if (slot && slot.visible) { // qmllint disable missing-property
                return true;
            }
        }
        return false;
    }
    /*! Every other module: the tab pages' modules, in the theme's order. */
    readonly property var bodyModules: root.theme.modules.filter(id => Modules.ABOVE_TABS.indexOf(id) < 0)
    /*! The pages this theme has at least one module for. */
    readonly property var populatedPages: Modules.PAGES.filter(
        page => root.bodyModules.some(id => Modules.page(id) === page))
    /*! The tab bar shows only when every page has modules. */
    readonly property bool tabsShown: root.populatedPages.length === Modules.PAGES.length
    readonly property string shownPage: root.tabsShown ? root.currentPage
        : (root.populatedPages.length > 0 ? root.populatedPages[0] : Modules.PAGES[0])

    /*! The module ids on one page. Without tabs, the one page holds them all. */
    function modulesOn(page) {
        if (!root.tabsShown) {
            return page === root.shownPage ? root.bodyModules : [];
        }
        return root.bodyModules.filter(id => Modules.page(id) === page);
    }

    function showPage(page) {
        if (Modules.PAGES.indexOf(page) >= 0) {
            root.currentPage = page;
        }
    }

    function pageTitle(page) {
        return page === "weather"
            ? i18ndc(root.domain, "Tab title: the weather page", "Weather")
            : i18ndc(root.domain, "Tab title: the brightness page", "Brightness");
    }

    /*! Live hint on the weather tab: the temperature, as the status line shows it. */
    readonly property string weatherHint: root.client.hasTemperature
        ? Format.tempText(root.client.temperatureC, root.client.isFahrenheit)
        : ""
    readonly property var tabEntries: Modules.PAGES.map(page => ({
        id: page,
        text: root.pageTitle(page),
        hint: page === "weather" ? root.weatherHint : "",
    }))

    Shortcut {
        sequence: "Ctrl+Tab"
        enabled: root.tabsShown
        onActivated: root.showPage(Modules.step(root.shownPage, 1))
    }
    Shortcut {
        sequence: "Ctrl+Shift+Tab"
        enabled: root.tabsShown
        onActivated: root.showPage(Modules.step(root.shownPage, -1))
    }

    /*! Screenshot harness only (tools/shot.sh --config "picker=open|trip|open-trip"):
        "open" opens the theme picker; "trip" makes a tab round trip (Brightness,
        Weather, Brightness) and then opens it; "open-trip" opens it, then makes the
        round trip while it is open. Empty outside the harness. */
    readonly property var harness: root.client.harnessConfig()

    Timer {
        id: harnessTimer

        property var steps: []
        property int next: 0

        interval: 800
        repeat: true
        running: next < steps.length && root.client.isConnected
        onTriggered: {
            steps[next]();
            next += 1;
        }
    }

    Component.onCompleted: {
        const trip = [() => root.showPage("weather"), () => root.showPage("brightness")];
        const open = () => themeButton.openPicker();
        const mode = String(root.harness.picker || "");
        if (mode === "open") {
            harnessTimer.steps = [open];
        } else if (mode === "trip") {
            harnessTimer.steps = trip.concat([open]);
        } else if (mode === "open-trip") {
            harnessTimer.steps = [open].concat(trip);
        }
    }

    /*! True while live or 5 s reconnect-grace cached daemon status is available. */
    readonly property bool hasLiveOrStale: root.client.isConnected || root.client.statusStale
    readonly property string offlineIcon: {
        switch (root.client.errorKind) {
        case SunReactorClient.PermissionDenied:
            return "dialog-password";
        case SunReactorClient.VersionMismatch:
            return "dialog-warning";
        case SunReactorClient.Timeout:
            return "chronometer";
        case SunReactorClient.Protocol:
            return "dialog-error";
        default:
            return "network-disconnect";
        }
    }
    readonly property string offlineTitle: {
        switch (root.client.errorKind) {
        case SunReactorClient.PermissionDenied:
            return i18nd(root.domain, "Permission denied");
        case SunReactorClient.VersionMismatch:
            return i18nd(root.domain, "Version mismatch");
        case SunReactorClient.Timeout:
            return i18nd(root.domain, "Connection timed out");
        case SunReactorClient.Protocol:
            return i18nd(root.domain, "Protocol error");
        default:
            return i18nd(root.domain, "Not connected");
        }
    }
    readonly property string offlineExplanation: {
        switch (root.client.errorKind) {
        case SunReactorClient.PermissionDenied:
            return i18nd(root.domain,
                "Cannot access the sunreactord control socket. Check socket permissions and your user session.");
        case SunReactorClient.VersionMismatch:
            return i18nd(root.domain,
                "The plasmoid and sunreactord use incompatible protocol versions. Update the plasmoid and daemon.");
        case SunReactorClient.Timeout:
            return i18nd(root.domain,
                "sunreactord did not answer in time. Check its status or restart it with:");
        case SunReactorClient.Protocol:
            return root.client.lastError.length > 0
                ? root.client.lastError
                : i18nd(root.domain, "Received an unexpected or malformed response from sunreactord.");
        default:
            return i18nd(root.domain,
                "The background service is not answering. Start it with:");
        }
    }
    readonly property bool showStartCommand: root.client.errorKind === SunReactorClient.None
        || root.client.errorKind === SunReactorClient.NotRunning
        || root.client.errorKind === SunReactorClient.Timeout

    contentItem: ColumnLayout {
        spacing: 0

        Connections {
            target: root.client

            function onPopupOpenChanged(): void {
                if (root.client.popupOpen) {
                    root.currentPage = root.startPage;
                } else {
                    root.closePopups();
                }
            }
        }

        // The header: the offline notice, the status line and the tab row. Its own
        // column, so its height does not wait on the pages' layout (popupHeight).
        ColumnLayout {
            id: headerColumn

            Layout.fillWidth: true
            Layout.leftMargin: root.edgeInset
            Layout.rightMargin: root.edgeInset
            spacing: 0

            SetupGuide {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.largeSpacing
                Layout.bottomMargin: Kirigami.Units.largeSpacing
                visible: !root.hasLiveOrStale
                pluginMissing: false
                iconName: root.offlineIcon
                title: root.offlineTitle
                explanation: root.offlineExplanation
                showStartCommand: root.showStartCommand
                showInstallCommand: root.showStartCommand
                domain: root.domain
            }

            Repeater {
                id: topRepeater

                model: root.topModules

                delegate: ModuleSlot {
                    enabled: root.client.isConnected
                    opacity: root.client.statusStale ? 0.45 : 1.0
                }
            }

            // The tab row: the tab bar (when the theme has both pages) and, at its
            // right end, the theme button. The row stays when the bar hides itself.
            RowLayout {
                id: tabRow

                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                visible: root.hasLiveOrStale
                enabled: root.client.isConnected
                opacity: root.client.statusStale ? 0.45 : 1.0
                spacing: 0

                Loader {
                    id: tabSlot

                    // Skin by atomic theme identity; no daemon theme means the plain bar.
                    readonly property string tabSource: Modules.tabBarSource(
                        root.theme.skinIdentity.available ? root.theme.skinIdentity.id : "",
                        root.theme.skinIdentity.available ? root.theme.skinIdentity.renderer : "")

                    function load() {
                        setSource(Qt.resolvedUrl(tabSource), { theme: root.theme });
                    }

                    Layout.fillWidth: true
                    visible: root.tabsShown && status === Loader.Ready

                    onTabSourceChanged: load()
                    Component.onCompleted: load()
                    onLoaded: {
                        // Each dynamically loaded tab bar has these properties.
                        item.tabs = Qt.binding(() => root.tabEntries); // qmllint disable missing-property
                        item.current = Qt.binding(() => root.shownPage); // qmllint disable missing-property
                    }

                    Connections {
                        target: tabSlot.item
                        ignoreUnknownSignals: true

                        function onPicked(id: string): void {
                            root.showPage(id);
                        }
                        function onDarkModeToggled(dark: bool): void {
                            Plasmoid.configuration.xboxDarkMode = dark; // qmllint disable missing-property
                        }
                        function onLightModeToggled(light: bool): void {
                            if (root.theme.themeId === "nothing") {
                                Plasmoid.configuration.nothingLightMode = light; // qmllint disable missing-property
                            } else if (root.theme.themeId === "macos") {
                                Plasmoid.configuration.macosLightMode = light; // qmllint disable missing-property
                            } else if (root.theme.themeId === "braun") {
                                Plasmoid.configuration.braunLightMode = light; // qmllint disable missing-property
                            }
                        }
                    }
                }

                // Pushes the lamp and the theme button to the right edge when the bar is hidden.
                Item {
                    Layout.fillWidth: true
                    visible: !tabSlot.visible
                }

                // The status lamp: the daemon state while the status line is hidden.
                StatusLamp {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.rightMargin: Kirigami.Units.smallSpacing
                    visible: !root.statusLineShown
                    theme: root.theme
                    client: root.client
                }

                ThemeButton {
                    id: themeButton

                    theme: root.theme
                    client: root.client
                    visible: root.hasLiveOrStale
                    // Plasma adds the main.xml configuration keys dynamically.
                    themeMode: Plasmoid.configuration.themeMode // qmllint disable missing-property
                    pinnedId: Plasmoid.configuration.pinnedTheme // qmllint disable missing-property
                    onFollowRequested: Plasmoid.configuration.themeMode = "follow" // qmllint disable missing-property
                    onPinRequested: id => {
                        Plasmoid.configuration.pinnedTheme = id; // qmllint disable missing-property
                        Plasmoid.configuration.themeMode = "pinned"; // qmllint disable missing-property
                    }
                }
            }
        }

        // The pages. All of them load, so a tab switch has the new page's size at
        // once; the shown page's column decides the popup height (popupHeight).
        Item {
            id: pages

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.topMargin: root.hasLiveOrStale ? root.theme.sectionSpacing : 0
            visible: root.hasLiveOrStale
            enabled: root.client.isConnected
            opacity: root.client.statusStale ? 0.45 : 1.0
            implicitHeight: {
                for (let i = 0; i < pageRepeater.count; ++i) {
                    // The delegates are PagePanels; the Repeater types them as items.
                    const panel = pageRepeater.itemAt(i);
                    if (panel && panel.modelData === root.shownPage) { // qmllint disable missing-property
                        return panel.columnHeight; // qmllint disable missing-property
                    }
                }
                return 0;
            }

            Repeater {
                id: pageRepeater

                model: Modules.PAGES

                delegate: PagePanel {
                    anchors.fill: parent
                    visible: modelData === root.shownPage
                }
            }
        }
    }

    // One tab page: its modules in the theme's order, scrolling on its own.
    component PagePanel: PlasmaComponents3.ScrollView {
        id: panel

        /*! The page id (Modules.PAGES). */
        required property string modelData
        /*! The page's height, from its column: what the popup measures. */
        readonly property real columnHeight: pageColumn.implicitHeight

        implicitHeight: columnHeight
        PlasmaComponents3.ScrollBar.horizontal.visible: false
        // The page scrolls up and down only: a sideways drift would shift its
        // content past the left edge.
        Binding {
            target: panel.contentItem
            property: "flickableDirection"
            value: Flickable.VerticalFlick
        }

        // A page opens at its top, not where it was left.
        onVisibleChanged: {
            if (visible) {
                PlasmaComponents3.ScrollBar.vertical.position = 0;
            }
        }

        // The content column, inset by the same margin on both sides inside the
        // viewport. Its width never depends on the scrollbar: one grid unit is
        // always kept for it, scrolling or not. A scrollbar that appears narrows
        // availableWidth, which re-wraps the text, which changes the page height
        // and so whether the page scrolls: a feedback loop (the handheld stall).
        ColumnLayout {
            id: pageColumn

            x: root.edgeInset
            width: panel.width - 2 * root.edgeInset - root.theme.gridUnit
            spacing: root.theme.sectionSpacing

            Repeater {
                model: root.modulesOn(panel.modelData)

                delegate: ModuleSlot {
                    pageSlot: true
                    pageCurrent: panel.modelData === root.shownPage
                }
            }

            // Comfortable bottom breathing room on the Weather page so cards
            // never sit flush against the popup bottom edge.
            Item {
                Layout.fillWidth: true
                implicitHeight: Math.round(root.theme.gridUnit * 0.55)
                visible: panel.modelData === "weather"
            }
        }
    }

    // One module's slot: the file for the theme's skin, shown while the module
    // reports itself available.
    component ModuleSlot: Loader {
        id: slot

        required property string modelData
        property bool pageSlot: false
        property bool pageCurrent: true
        // The file depends on the atomic skinIdentity (id + renderer together),
        // so a theme change never sees a torn (oldId, newRenderer) pair and never
        // speculatively loads skins/instrument while a theme is pending.
        readonly property string moduleSource: (root.theme.skinIdentity.available && Modules.file(modelData).length > 0)
            ? Modules.source(modelData, root.theme.skinIdentity.id, root.theme.skinIdentity.renderer) : ""

        function load() {
            if (active && moduleSource.length > 0) {
                setSource(Qt.resolvedUrl(moduleSource), {
                    theme: root.theme,
                    client: root.client,
                    settings: moduleSettings,
                });
            } else {
                source = "";
            }
        }

        Layout.fillWidth: true
        // No module widens the page: a slot is at most its column's width, and
        // whatever a module paints past its slot is clipped.
        Layout.maximumWidth: parent ? parent.width : 0
        clip: true
        // Every module exposes `available` (modules/Modules.js); the
        // loaded type is only known at run time.
        visible: status === Loader.Ready && item !== null && item.available === true // qmllint disable missing-property
        active: moduleSource.length > 0 && (!pageSlot || (root.client.popupOpen && pageCurrent))

        onModuleSourceChanged: load()
        onActiveChanged: load()
        Component.onCompleted: load()
    }

    footer: PopupFooter {
        id: popupFooter

        client: root.client
        theme: root.theme
        visible: root.hasLiveOrStale && popupFooter.hasParts
        enabled: root.client.isConnected
        opacity: root.client.statusStale ? 0.45 : 1.0
    }
}
