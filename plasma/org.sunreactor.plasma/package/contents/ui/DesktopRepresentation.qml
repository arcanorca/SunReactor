/*
 * Live, freely scalable KDE Plasma Desktop representation of SunReactor.
 *
 * Hosts the full interactive skin inside a proportional scaled viewport so the
 * user can resize the desktop widget smoothly from a compact tile up to a
 * large dashboard without distorting internal proportions or causing tab-switch
 * layout jitter.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.sunreactor.plasma

Item {
    id: root

    required property SunReactorClient client

    readonly property alias theme: inner.theme
    readonly property alias harness: inner.harness
    readonly property alias harnessWindow: inner.harnessWindow
    readonly property alias popupHovered: inner.popupHovered
    readonly property alias activePopupMenus: inner.activePopupMenus
    property alias currentPage: inner.currentPage

    readonly property real baseWidth: Math.max(1, inner.popupWidth)
    // Latch the tallest observed full-page height (initially ~36 gu / popupHeight) so a
    // tall desktop widget renders without a vertical scrollbar AND tab switches never shift zoom.
    property real referenceHeight: Math.max(inner.popupMinHeight, inner.theme.gridUnit * 28)
    readonly property real zoom: root.width > 0 && root.height > 0
        ? Math.max(0.55, Math.min(2.5, Math.min(root.width / baseWidth, root.height / referenceHeight)))
        : 1.0

    Connections {
        target: inner

        function onPopupHeightChanged(): void {
            if (inner.popupHeight > root.referenceHeight) {
                root.referenceHeight = inner.popupHeight;
            }
        }
    }

    function showPage(page: string): void {
        inner.showPage(page);
    }

    function closePopups(): void {
        inner.closePopups();
    }

    Layout.minimumWidth: Math.round(inner.theme.gridUnit * 10)
    Layout.minimumHeight: Math.round(inner.theme.gridUnit * 10)
    Layout.preferredWidth: inner.popupWidth
    Layout.preferredHeight: inner.popupHeight
    Layout.maximumWidth: Number.POSITIVE_INFINITY
    Layout.maximumHeight: Number.POSITIVE_INFINITY
    clip: true

    FullRepresentation {
        id: inner

        client: root.client
        desktopEmbedded: true
        x: 0
        y: 0
        width: root.width > 0 ? Math.max(1, root.width / root.zoom) : inner.popupWidth
        height: root.height > 0 ? Math.max(1, root.height / root.zoom) : inner.popupHeight
        transformOrigin: Item.TopLeft
        scale: root.zoom
    }
}
