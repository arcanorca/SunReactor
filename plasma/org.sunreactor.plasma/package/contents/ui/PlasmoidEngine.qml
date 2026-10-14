/*
 * A ThemeEngine bound to this plasmoid instance's Appearance settings
 * (main.xml): follow / pin and the local overrides. The panel item and the
 * popup each hold one; they share the client, so a pin is fetched once.
 */

import QtQuick
import org.kde.plasma.plasmoid

import "engine"

ThemeEngine {
    mode: Plasmoid.configuration.themeMode
    pinnedId: Plasmoid.configuration.pinnedTheme
    accentOverride: Plasmoid.configuration.accentOverride
    densityOverride: Plasmoid.configuration.densityOverride
    rendererOverride: Plasmoid.configuration.rendererOverride
    pixelScaleOverride: Plasmoid.configuration.pixelScale
    weatherIconStyleOverride: Plasmoid.configuration.weatherIconStyle
    modulesOverride: Plasmoid.configuration.moduleOrder
    hiddenModules: Plasmoid.configuration.hiddenModules
}
