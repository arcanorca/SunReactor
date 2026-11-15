/*
 * The daemon's mode as the one capitalised word the status lines show: LIVE,
 * IDLE, MANUAL, PAUSED, OFFLINE. Kept in one place so the Telemetry skins do
 * not each carry (and translate) their own copy of the switch.
 */

import QtQuick
import org.sunreactor.plasma

QtObject {
    id: root

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"

    /*! The word for a SunReactorClient.Mode. */
    function label(mode) {
        switch (mode) {
        case SunReactorClient.Offline:
            return i18ndc(root.domain, "Daemon state in the status line", "OFFLINE");
        case SunReactorClient.Paused:
            return i18ndc(root.domain, "Daemon state in the status line", "PAUSED");
        case SunReactorClient.Manual:
            return i18ndc(root.domain, "Daemon state in the status line", "MANUAL");
        case SunReactorClient.IdleDimmed:
            return i18ndc(root.domain, "Daemon state in the status line", "IDLE");
        default:
            return i18ndc(root.domain, "Daemon state in the status line", "LIVE");
        }
    }
}
