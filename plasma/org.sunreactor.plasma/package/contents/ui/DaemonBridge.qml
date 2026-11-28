/*
 * C++ IPC client loader bridge: loaded via Loader in main.qml so KDE Store
 * users without the native plugin yet see SetupGuide instead of a red QML error.
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

Item {
    id: bridge

    property alias client: daemon

    SunReactorClient {
        id: daemon
    }
}
