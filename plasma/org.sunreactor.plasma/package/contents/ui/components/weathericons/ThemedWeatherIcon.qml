/*
 * The weather icon of the current theme: the file the registry names for the
 * theme's id or renderer (Modules.weatherIcon), loaded with the condition, day or
 * night, size and surface colours, and kept live. A theme change swaps the file.
 *
 *   ThemedWeatherIcon { theme: view.theme; condition: "rain"; night: false; size: 22 }
 *
 * The file is set up with setSource, so its required properties are given at
 * creation; the bindings after that keep it current.
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

    /*! The theme's file, relative to ui/. */
    readonly property string iconPath: Modules.weatherIcon(theme.available ? theme.themeId : "", theme.renderer)

    width: size
    height: size
    implicitWidth: size
    implicitHeight: size

    Loader {
        id: slot

        anchors.fill: parent
        readonly property url iconUrl: Qt.resolvedUrl("../../" + root.iconPath)

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

        onIconUrlChanged: load()
        Component.onCompleted: load()

        onLoaded: {
            item.condition = Qt.binding(() => root.condition); // qmllint disable missing-property
            item.night = Qt.binding(() => root.night); // qmllint disable missing-property
            item.size = Qt.binding(() => root.size); // qmllint disable missing-property
            item.surface = Qt.binding(() => root.surface); // qmllint disable missing-property
            item.ink = Qt.binding(() => root.ink); // qmllint disable missing-property
        }
    }
}
