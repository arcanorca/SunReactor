/*
 * The popup footer: a thin container. Its parts - the theme chip and Update
 * now - are loaded through footer/Parts.js, the same skin lookup the modules
 * use, so a family skin can replace any one of them.
 *
 * Chrome comes from the theme: the theme background a step lighter, a hairline
 * in the theme's line colour on top. Without a daemon theme it is Plasma's own
 * footer heading.
 *
 * The footer never writes settings itself: it asks with followRequested() /
 * pinRequested(id) and FullRepresentation stores the choice in the plasmoid
 * config (per instance; the daemon config is never touched).
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.extras as PlasmaExtras
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"
import "footer/Parts.js" as Parts

Item {
    id: root

    required property SunReactorClient client
    required property Theme theme
    /*! No longer read: pausing moved to the Automation module. Kept because
        FullRepresentation still passes it, and that file is not edited here. */
    property int defaultPauseMinutes: 60
    /*! Plasmoid config: "follow" or "pinned", and the pinned id. */
    property string themeMode: "follow"
    property string pinnedId: ""
    /*! Why a pin is not in effect (ThemeEngine.notice). */
    property string pinNotice: ""

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property bool pinned: themeMode === "pinned" && pinnedId.length > 0
    /*! Name of the theme the daemon is on (what "follow" means right now). */
    readonly property string followedName: client.theme && client.theme.name ? client.theme.name : client.themeName

    signal followRequested()
    signal pinRequested(string id)

    implicitHeight: row.implicitHeight + 2 * Kirigami.Units.smallSpacing
    implicitWidth: row.implicitWidth + 2 * Kirigami.Units.smallSpacing

    PlasmaExtras.PlasmoidHeading {
        anchors.fill: parent
        visible: !root.theme.available
        position: QQC2.ToolBar.Footer
    }

    Rectangle {
        anchors.fill: parent
        visible: root.theme.available
        color: root.theme.isPixel ? root.theme.bg : root.theme.raised

        Rectangle {
            width: parent.width
            height: root.theme.isPixel ? root.theme.grid.unit : root.theme.lineWidth
            color: root.theme.line
        }
    }

    RowLayout {
        id: row

        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: Parts.ORDER

            delegate: Loader {
                id: part

                required property var modelData

                readonly property string path: modelData ? Parts.source(modelData, root.theme.themeId,
                                                                        root.theme.renderer) : ""

                function load() {
                    if (path.length > 0) {
                        setSource(Qt.resolvedUrl("../" + path), { footer: root });
                    } else {
                        source = "";
                    }
                }

                Layout.fillWidth: !modelData
                Layout.alignment: Qt.AlignVCenter
                onPathChanged: load()
                Component.onCompleted: load()
            }
        }
    }
}
