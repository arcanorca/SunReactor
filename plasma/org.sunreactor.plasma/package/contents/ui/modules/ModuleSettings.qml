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

    /*! Shared display labels/notices so every skin shares one translation call. */
    readonly property string allDisplaysLabel: i18nd(domain, "All displays")
    readonly property string contestedNotice: i18ndc(domain, "Display notice when another app owns DDC",
                                                     "Paused: another app is changing this display")
    readonly property string deferredNotice: i18ndc(domain, "Display notice when a write is queued",
                                                    "Queued, applying shortly")
    readonly property string pausedTag: i18ndc(domain, "Compact tag when another app is changing this display", "Paused")
    readonly property string queuedTag: i18ndc(domain, "Compact tag when a write is queued", "Queued")
}
