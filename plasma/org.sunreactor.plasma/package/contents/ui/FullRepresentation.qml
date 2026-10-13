/*
 * The popup: an instrument, composed of modules in the order the theme lists
 * them (theme.widget.modules; see modules/Modules.js for the contract).
 *
 * Structure follows Plasma's own brightness applet - a Representation holding
 * a scrollable column of flat rows, with a footer for the actions. There is no
 * title: the popup does not repeat the name of the widget. When a daemon theme
 * is in effect the content sits on the theme's background.
 *
 * Telemetry is the status line and stays on top. When the theme lists modules
 * for both tab pages (Modules.PAGES), a tab bar under it switches between the
 * pages; each page scrolls on its own and the footer stays put. The popup is as
 * tall as the page on screen needs (see popupHeight).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.plasma.extras as PlasmaExtras
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "components"
import "engine"
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
    }

    ModuleSettings {
        id: moduleSettings

        overrideMinutes: Plasmoid.configuration.overrideDurationMinutes
        defaultSuspendMinutes: Plasmoid.configuration.defaultSuspendMinutes
        showSolarElevation: Plasmoid.configuration.showSolarElevation
    }

    Layout.minimumWidth: Kirigami.Units.gridUnit * 16
    Layout.preferredWidth: Kirigami.Units.gridUnit * 22
    Layout.maximumWidth: Kirigami.Units.gridUnit * 40
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
    readonly property real popupMinHeight: Kirigami.Units.gridUnit * 14
    /*! The largest popup: 75% of the screen's available height, at most 46 grid
        units (46 alone until the screen is known). */
    readonly property real popupMaxHeight: Math.max(root.popupMinHeight, Math.min(
        Kirigami.Units.gridUnit * 46,
        Screen.desktopAvailableHeight > 0 ? Screen.desktopAvailableHeight * 0.75 : Kirigami.Units.gridUnit * 46))
    /*! Status line, tab row and the gap above the pages. */
    readonly property real headerHeight: headerColumn.implicitHeight + pages.Layout.topMargin
    /*! The shown page's column. */
    readonly property real pageHeight: pages.implicitHeight
    /*! The footer and the popup's own top and bottom margins. */
    readonly property real chromeHeight: (popupFooter.visible ? popupFooter.implicitHeight : 0)
                                         + root.topPadding + root.bottomPadding
    readonly property real popupHeight: Math.min(root.popupMaxHeight,
        Math.max(root.popupMinHeight, root.headerHeight + root.pageHeight + root.chromeHeight))

    collapseMarginsHint: true
    // collapseMarginsHint extends the content 6 px past each side of the popup
    // (into the Plasma frame's margins). This widget paints its own background
    // over the popup's full width, so the sides stay inside it: the content is
    // exactly the popup's width.
    leftPadding: 0
    rightPadding: 0

    background: Frame {
        theme: root.theme
        visible: root.theme.available
        pixel: false
        fillColor: root.theme.bg
        color: root.theme.line
        style: root.theme.border === "none" ? "none" : "line"
    }

    // --- tab pages -------------------------------------------------------------

    /*! Screenshot harness only (tools/shot.sh --config "tab=weather"): empty otherwise. */
    readonly property string harnessPage: String(client.harnessConfig().tab || "")
    /*! The page the popup starts on each time it opens. */
    readonly property string startPage: Modules.PAGES.indexOf(harnessPage) >= 0 ? harnessPage : Modules.PAGES[0]
    /*! The page the user picked. Kept while the popup is open; in memory only. */
    property string currentPage: startPage
    // A page change can move the tab row and the popup window: re-hang the theme picker.
    onCurrentPageChanged: themeButton.relayout()
    onTabsShownChanged: themeButton.relayout()

    /*! The status line: modules that stay above the tab bar, in the theme's order. */
    readonly property var topModules: root.theme.modules.filter(id => Modules.ABOVE_TABS.indexOf(id) >= 0)
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
        ? i18ndc(root.domain, "Temperature in degrees Celsius, compact", "%1°C", Math.round(root.client.temperatureC))
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

    contentItem: ColumnLayout {
        spacing: 0

        Connections {
            target: root.client

            function onPopupOpenChanged(): void {
                if (root.client.popupOpen) {
                    root.currentPage = root.startPage;
                }
            }
        }

        // The header: the offline notice, the status line and the tab row. Its own
        // column, so its height does not wait on the pages' layout (popupHeight).
        ColumnLayout {
            id: headerColumn

            Layout.fillWidth: true
            spacing: 0

            PlasmaExtras.PlaceholderMessage {
                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.gridUnit
                Layout.bottomMargin: Kirigami.Units.gridUnit
                visible: !root.client.isConnected
                iconName: "network-disconnect"
                text: i18nd(root.domain, "Not connected")
                explanation: i18nd(root.domain,
                    "The background service is not answering. Start it with:\nsystemctl --user start sunreactord")
            }

            Repeater {
                model: root.topModules

                delegate: ModuleSlot {}
            }

            // The tab row: the tab bar (when the theme has both pages) and, at its
            // right end, the theme button. The row stays when the bar hides itself.
            RowLayout {
                id: tabRow

                Layout.fillWidth: true
                Layout.topMargin: Kirigami.Units.smallSpacing
                Layout.leftMargin: root.theme.available ? Kirigami.Units.smallSpacing : 0
                Layout.rightMargin: Layout.leftMargin
                visible: root.tabsShown || root.client.isConnected
                spacing: 0

                Loader {
                    id: tabSlot

                    // Skin by theme and renderer; no daemon theme means the plain bar.
                    readonly property string tabSource: Modules.tabBarSource(
                        root.theme.available ? root.theme.themeId : "",
                        root.theme.available ? root.theme.renderer : "")

                    function load() {
                        setSource(Qt.resolvedUrl(tabSource), { theme: root.theme });
                    }

                    Layout.fillWidth: true
                    visible: root.tabsShown && status === Loader.Ready

                    onTabSourceChanged: load()
                    Component.onCompleted: load()
                    onLoaded: {
                        item.tabs = Qt.binding(() => root.tabEntries);
                        item.current = Qt.binding(() => root.shownPage);
                    }

                    Connections {
                        target: tabSlot.item
                        ignoreUnknownSignals: true

                        function onPicked(id: string): void {
                            root.showPage(id);
                        }
                    }
                }

                // Pushes the theme button to the right edge when the bar is hidden.
                Item {
                    Layout.fillWidth: true
                    visible: !tabSlot.visible
                }

                ThemeButton {
                    id: themeButton

                    theme: root.theme
                    client: root.client
                    visible: root.client.isConnected
                    themeMode: Plasmoid.configuration.themeMode
                    pinnedId: Plasmoid.configuration.pinnedTheme
                    onFollowRequested: Plasmoid.configuration.themeMode = "follow"
                    onPinRequested: id => {
                        Plasmoid.configuration.pinnedTheme = id;
                        Plasmoid.configuration.themeMode = "pinned";
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
            Layout.topMargin: root.theme.sectionSpacing
            implicitHeight: {
                for (let i = 0; i < pageRepeater.count; ++i) {
                    // The delegates are PagePanels; the Repeater types them as items.
                    const panel = pageRepeater.itemAt(i);
                    if (panel.modelData === root.shownPage) { // qmllint disable missing-property
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

        // A page opens at its top, not where it was left.
        onVisibleChanged: {
            if (visible) {
                PlasmaComponents3.ScrollBar.vertical.position = 0;
            }
        }

        ColumnLayout {
            id: pageColumn

            width: panel.availableWidth
            spacing: root.theme.sectionSpacing

            Repeater {
                model: root.modulesOn(panel.modelData)

                delegate: ModuleSlot {}
            }
        }
    }

    // One module's slot: the file for the theme's skin, shown while the module
    // reports itself available.
    component ModuleSlot: Loader {
        id: slot

        required property string modelData
        // The file depends on the theme (skins per renderer / id),
        // so a theme change can swap it after creation.
        readonly property string moduleSource: Modules.file(modelData).length > 0
            ? Modules.source(modelData, root.theme.themeId, root.theme.renderer) : ""

        function load() {
            if (moduleSource.length > 0) {
                setSource(Qt.resolvedUrl(moduleSource), {
                    theme: root.theme,
                    client: root.client,
                    settings: moduleSettings,
                });
            }
        }

        Layout.fillWidth: true
        Layout.leftMargin: root.theme.available ? Kirigami.Units.smallSpacing : 0
        Layout.rightMargin: Layout.leftMargin
        // No module widens the page: a slot is at most its column's width, and
        // whatever a module paints past its slot is clipped.
        Layout.maximumWidth: parent ? parent.width - Layout.leftMargin - Layout.rightMargin : 0
        clip: true
        // Every module exposes `available` (modules/Modules.js); the
        // loaded type is only known at run time.
        visible: status === Loader.Ready && item.available === true // qmllint disable missing-property
        active: moduleSource.length > 0

        onModuleSourceChanged: load()
        Component.onCompleted: load()
    }

    footer: PopupFooter {
        id: popupFooter

        client: root.client
        theme: root.theme
        defaultPauseMinutes: Plasmoid.configuration.defaultSuspendMinutes
        visible: root.client.isConnected
    }
}
