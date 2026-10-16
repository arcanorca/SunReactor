/*
 * Resolves which theme this plasmoid instance shows (DESIGN.md section 4,
 * follow / pin / override):
 *
 *   follow  - client.theme, i.e. whatever the TUI saved (the default);
 *   pinned  - client.pinnedTheme, fetched with get_theme for `pinnedId`;
 *             while it is pending, missing or unsupported (older daemon)
 *             the engine keeps following and says why in `notice`;
 *   overrides (accent, density, renderer, pixelScale, modules, weather icon
 *   style) on top.
 *
 * The widget never writes the daemon's config: pinning is local to this
 * instance. Several engines may share one client (popup and panel each have
 * one); they all bind the same pinnedThemeId value into it.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: root

    required property SunReactorClient client

    /*! "follow" or "pinned" (plasmoid config themeMode). */
    property string mode: "follow"
    /*! Theme id to pin when mode is "pinned" (plasmoid config pinnedTheme). */
    property string pinnedId: ""
    property string accentOverride: ""
    property string densityOverride: ""
    property string rendererOverride: ""
    property int pixelScaleOverride: 0
    property int uiScaleOverride: 0
    /*! "auto", "modern", "line", "solid" or "pixel" (plasmoid config weatherIconStyle). */
    property string weatherIconStyleOverride: ""
    property var modulesOverride: []
    /*! Module ids to leave out (plasmoid config hiddenModules). */
    property var hiddenModules: []

    /*! The resolved tokens every module reads. */
    readonly property Theme theme: resolved

    readonly property bool pinning: mode === "pinned" && pinnedId.length > 0
    readonly property bool usingPinned: pinning && client.pinnedTheme
        && client.pinnedTheme.palette !== undefined
    /*! Why a pin is not in effect: "", "pending", "unsupported", "not_found", "offline". */
    readonly property string notice: pinning && !usingPinned ? (client.pinnedThemeError || "pending") : ""

    visible: false
    width: 0
    height: 0

    Binding {
        target: root.client
        property: "pinnedThemeId"
        value: root.pinning ? root.pinnedId : ""
    }

    Theme {
        id: resolved

        spec: root.usingPinned ? root.client.pinnedTheme : root.client.theme
        source: root.usingPinned ? "pinned" : (root.client.theme && root.client.theme.palette ? "follow" : "plasma")
        overrides: ({
            accent: root.accentOverride,
            density: root.densityOverride,
            renderer: root.rendererOverride,
            pixelScale: root.pixelScaleOverride,
            uiScale: root.uiScaleOverride,
            weatherIconStyle: root.weatherIconStyleOverride,
            modules: root.modulesOverride,
            hidden: root.hiddenModules,
        })
    }
}
