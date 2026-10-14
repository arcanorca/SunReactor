/*
 * The weather icon of a surface, drawn in the style the user chose
 * (Theme.weatherIconStyle, the "Weather icons" setting):
 *
 *   auto    the theme's own icon: the caller's drawing when it gives one (its
 *           children, below), else the registry file for the theme (Modules.weatherIcon).
 *   modern  ModernWeatherIcon: full colour, the same in every theme.
 *   line    WeatherGlyph outline; solid: WeatherGlyph filled (GlyphIcon).
 *   pixel   PixelIcon: the 16 px sprites (the DMG's atlas on the Game Boy).
 *
 * An explicit style wins over the theme's registry entry; auto keeps every skin as
 * it was. A skin that draws its own icon passes that drawing as children, which
 * are shown in auto only:
 *
 *   ThemedWeatherIcon {
 *       theme: root.theme; condition: "rain"; night: false; size: 22
 *       WeatherGlyph { anchors.centerIn: parent; ... }   // the skin's own auto drawing
 *   }
 *
 * The registry file is set up with setSource, so its required properties are given
 * at creation; the bindings after that keep it current.
 */

pragma ComponentBehavior: Bound

import QtQuick
import "../../modules/Modules.js" as Modules

Item {
    id: root

    required property var theme
    required property string condition
    required property bool night
    /*! Logical px, square. */
    property real size: 22
    /*! The colours the icon sits on (the panel passes its own; default theme bg and fg). */
    property color surface: theme.bg
    property color ink: theme.fg

    /*! The style in effect: "auto", "modern", "line", "solid" or "pixel". */
    readonly property string style: theme.weatherIconStyle
    readonly property bool isAuto: style === "auto"
    /*! The theme's file, relative to ui/. */
    readonly property string iconPath: Modules.weatherIcon(theme.available ? theme.themeId : "", theme.renderer)

    /*! The caller's own auto drawing: children are shown while the style is auto. */
    default property alias autoArt: autoSlot.data

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    Item {
        id: autoSlot

        anchors.fill: parent
        visible: root.isAuto
    }

    Loader {
        id: slot

        anchors.fill: parent
        // The registry file only when the caller drew no icon of its own.
        active: root.isAuto && autoSlot.data.length === 0
        readonly property url iconUrl: Qt.resolvedUrl("../../" + root.iconPath)

        Component.onCompleted: if (active) load()

        function load() {
            setSource(iconUrl, {
                condition: root.condition,
                night: root.night,
                theme: root.theme,
                size: root.size,
                surface: root.surface,
                ink: root.ink,
            });
        }

        onActiveChanged: { if (active) load(); }
        onIconUrlChanged: { if (active) load(); }

        onLoaded: {
            item.condition = Qt.binding(() => root.condition); // qmllint disable missing-property
            item.night = Qt.binding(() => root.night); // qmllint disable missing-property
            item.size = Qt.binding(() => root.size); // qmllint disable missing-property
            item.surface = Qt.binding(() => root.surface); // qmllint disable missing-property
            item.ink = Qt.binding(() => root.ink); // qmllint disable missing-property
        }
    }

    Loader {
        anchors.fill: parent
        active: root.style === "modern"
        sourceComponent: Component {
            ModernWeatherIcon {
                theme: root.theme
                condition: root.condition
                night: root.night
                size: root.size
                surface: root.surface
                ink: root.ink
            }
        }
    }

    Loader {
        anchors.fill: parent
        active: root.style === "line" || root.style === "solid"
        sourceComponent: Component {
            GlyphIcon {
                theme: root.theme
                condition: root.condition
                night: root.night
                size: root.size
                surface: root.surface
                ink: root.ink
            }
        }
    }

    Loader {
        anchors.fill: parent
        active: root.style === "pixel"
        sourceComponent: Component {
            PixelIcon {
                // A pixel theme shows its own atlas frames; other themes the shared sprites.
                atlas: root.theme.isPixel
                theme: root.theme
                condition: root.condition
                night: root.night
                size: root.size
                surface: root.surface
                ink: root.ink
            }
        }
    }
}
