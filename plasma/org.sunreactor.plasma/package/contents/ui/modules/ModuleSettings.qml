/*
 * The plasmoid settings modules may read, passed in by FullRepresentation so
 * that no module reaches for Plasmoid.configuration itself.
 */

import QtQuick

QtObject {
    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    /*! How long a brightness set by hand lasts; 0 = until undone. */
    property int overrideMinutes: 60
    /*! Duration of the plain "Pause" action; 0 = until resumed. */
    property int defaultSuspendMinutes: 60
    /*! Spell out the sun's height (opt-in). */
    property bool showSolarElevation: false
}
