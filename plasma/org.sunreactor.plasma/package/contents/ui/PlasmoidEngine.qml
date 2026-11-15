/*
 * A ThemeEngine bound to this plasmoid instance's Appearance settings
 * (main.xml): follow / pin and the local overrides. The panel item and the
 * popup each hold one; they share the client, so a pin is fetched once.
 */

import QtQuick
import org.kde.plasma.plasmoid

import "engine"

ThemeEngine {
    // Plasma adds the main.xml configuration keys dynamically.
    mode: Plasmoid.configuration.themeMode // qmllint disable missing-property
    pinnedId: Plasmoid.configuration.pinnedTheme // qmllint disable missing-property
    accentOverride: Plasmoid.configuration.accentOverride // qmllint disable missing-property
    densityOverride: Plasmoid.configuration.densityOverride // qmllint disable missing-property
    rendererOverride: Plasmoid.configuration.rendererOverride // qmllint disable missing-property
    pixelScaleOverride: Plasmoid.configuration.pixelScale // qmllint disable missing-property
    weatherIconStyleOverride: Plasmoid.configuration.weatherIconStyle // qmllint disable missing-property
    modulesOverride: Plasmoid.configuration.moduleOrder // qmllint disable missing-property
    hiddenModules: Plasmoid.configuration.hiddenModules // qmllint disable missing-property
    uiScaleOverride: Plasmoid.configuration.uiScale // qmllint disable missing-property
}
