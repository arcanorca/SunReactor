/*
 * The theme picker: the popover under ThemeButton. Its first row is always
 * "Follow SunReactor", with the terminal app's theme named under it and a check
 * while following. Below it, one row per daemon theme, grouped by family under
 * small-caps captions: a four-colour strip, the name, and a check on the pinned
 * one. Choosing a row asks for follow or pin; the widget stores the choice.
 * Keyboard: Up and Down move, Return / Enter / Space choose, Escape closes.
 * All text is small, in the theme's body face.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import "../engine"

QQC2.Popup {
    id: root

    /*! The button the popover hangs from (its Popup parent). */
    required property Item anchorItem
    required property Theme theme
    required property SunReactorClient client
    required property string domain
    /*! The theme the daemon is on, named under the follow row. */
    required property string followedName
    required property bool pinned
    required property string pinnedId

    signal followRequested()
    signal pinRequested(string id)

    readonly property bool listSupported: client.themeRegistry !== SunReactorClient.CapabilityUnsupported
    /*! Small text: the theme's body face (its bitmap face is for the tab bar), at most 11 px. */
    readonly property string textFamily: theme.available && theme.bodyFont.length > 0
        ? theme.bodyFont : Kirigami.Theme.defaultFont.family
    readonly property int textPx: 11
    readonly property int captionPx: 10
    /*! Largest popover height: 60% of the popup, and never past the window's bottom edge. */
    property real maxHeight: 0
    /*! The height the content wants, unclipped (the list's full length included). */
    readonly property real naturalHeight: contentLayout.implicitHeight + topPadding + bottomPadding

    width: 0
    height: Math.min(naturalHeight, maxHeight)
    // The content fills the capped popover, so the list gets the rest and scrolls.
    contentWidth: width - leftPadding - rightPadding
    contentHeight: height - topPadding - bottomPadding
    padding: Kirigami.Units.smallSpacing
    modal: false
    focus: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutsideParent

    function familyLabel(family) {
        switch (family) {
        case "pixel":
            return i18ndc(domain, "Theme family", "Pixel");
        case "terminal":
            return i18ndc(domain, "Theme family", "Terminal");
        default:
            return i18ndc(domain, "Theme family", "Instrument");
        }
    }

    /*! Hangs the popover under the anchor, its right edge on the anchor's, and
        keeps it inside the popup window. The anchor's window position is read
        afresh on every call, so the popover is right after any change: on open,
        and whenever the anchor, its parents or the window move or resize. */
    function place() {
        const win = anchorItem.Window;
        if (!win) {
            return;
        }
        const origin = anchorItem.mapToItem(null, 0, 0);
        const edge = Kirigami.Units.smallSpacing;
        const top = origin.y + anchorItem.height + edge;
        const wide = Math.max(0, Math.min(Kirigami.Units.gridUnit * 18, win.width - 2 * edge));
        const left = Math.max(edge, Math.min(origin.x + anchorItem.width - wide, win.width - edge - wide));
        root.width = wide;
        root.x = left - origin.x;
        root.y = anchorItem.height + edge;
        root.maxHeight = Math.max(Kirigami.Units.gridUnit * 4,
                                  Math.min(win.height * 0.6, win.height - top - edge));
    }

    function relayoutIfOpen() {
        if (root.opened) {
            root.place();
        }
    }

    function rebuild() {
        choices.clear();
        const order = ["pixel", "instrument", "terminal"];
        const sorted = client.themeList.slice().sort((a, b) =>
            (order.indexOf(a.family) - order.indexOf(b.family)) || String(a.name).localeCompare(String(b.name)));
        for (const t of sorted) {
            choices.append({ themeId: t.id, name: t.name, family: t.family,
                             bg: t.bg, fg: t.fg, accent: t.accent, secondary: t.secondaryAccent });
        }
        let current = 0;
        for (let i = 0; i < choices.count; ++i) {
            if (root.pinned && choices.get(i).themeId === root.pinnedId) {
                current = i;
            }
        }
        list.currentIndex = current;
        scrollToCurrent();
    }

    /*! Brings the current row into view once the list has laid out its rows. */
    function scrollToCurrent() {
        Qt.callLater(() => list.positionViewAtIndex(list.currentIndex, ListView.Contain));
    }

    function chooseFollow() {
        root.followRequested();
        root.close();
    }

    function choose(index) {
        const entry = choices.get(index);
        if (!entry) {
            return;
        }
        root.pinRequested(entry.themeId);
        root.close();
    }

    onAboutToShow: {
        client.refreshThemeList();
        place();
        rebuild();
    }
    onOpened: {
        place();
        root.scrollToCurrent();
        if (root.pinned) {
            list.forceActiveFocus();
        } else {
            followRow.forceActiveFocus();
        }
    }

    // The anchor, its parents and the window: any move or resize re-hangs the popover.
    Connections {
        target: root.anchorItem
        function onXChanged() { root.relayoutIfOpen(); }
        function onYChanged() { root.relayoutIfOpen(); }
        function onWidthChanged() { root.relayoutIfOpen(); }
        function onHeightChanged() { root.relayoutIfOpen(); }
    }
    Connections {
        target: root.anchorItem ? root.anchorItem.parent : null
        function onXChanged() { root.relayoutIfOpen(); }
        function onYChanged() { root.relayoutIfOpen(); }
    }
    Connections {
        target: root.anchorItem ? root.anchorItem.Window : null
        function onWidthChanged() { root.relayoutIfOpen(); }
        function onHeightChanged() { root.relayoutIfOpen(); }
    }
    Connections {
        target: root.client
        function onThemeListChanged() {
            if (root.opened) {
                root.rebuild();
            }
        }
    }

    ListModel {
        id: choices
    }

    background: Rectangle {
        color: root.theme.bg
        radius: root.theme.radius
        border.width: root.theme.isPixel ? root.theme.grid.unit : root.theme.lineWidth
        border.color: root.theme.borderActive
    }

    contentItem: ColumnLayout {
        id: contentLayout

        spacing: 0

        // The daemon fell back to another theme: say why, once, at the top.
        RowLayout {
            Layout.fillWidth: true
            Layout.bottomMargin: Kirigami.Units.smallSpacing
            visible: root.client.themeWarning.length > 0
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Icon {
                Layout.alignment: Qt.AlignTop
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: implicitWidth
                source: "dialog-warning"
                isMask: true
                color: root.theme.warning
            }
            PlasmaComponents3.Label {
                Layout.fillWidth: true
                text: root.client.themeWarning
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
                color: root.theme.fg
                font.family: root.textFamily
                font.pixelSize: root.textPx
            }
        }

        // Always the first row: follow the terminal app. Kept out of the list so
        // it never scrolls away.
        QQC2.ItemDelegate {
            id: followRow

            Layout.fillWidth: true
            focusPolicy: Qt.StrongFocus
            hoverEnabled: true
            Accessible.role: Accessible.RadioButton
            Accessible.name: i18ndc(root.domain, "Placeholder is the theme name", "Follow SunReactor (%1)", root.followedName)
            Accessible.checkable: true
            Accessible.checked: !root.pinned

            Keys.onDownPressed: list.forceActiveFocus()
            Keys.onReturnPressed: root.chooseFollow()
            Keys.onEnterPressed: root.chooseFollow()
            onClicked: root.chooseFollow()

            background: Rectangle {
                radius: root.theme.radius
                color: followRow.activeFocus || followRow.hovered
                    ? root.theme.alpha(root.theme.accent, 0.22)
                    : root.theme.alpha(root.theme.accent, root.pinned ? 0.06 : 0.14)
                border.width: root.theme.lineWidth
                border.color: root.theme.alpha(root.theme.accent, root.pinned ? 0 : 0.5)
            }

            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: implicitWidth
                    source: root.pinned ? "" : "checkmark"
                    isMask: true
                    color: root.theme.accent
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: i18nd(root.domain, "Follow SunReactor")
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: root.theme.fg
                        font.family: root.textFamily
                        font.pixelSize: root.textPx
                        font.weight: Font.DemiBold
                    }
                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        visible: root.followedName.length > 0
                        text: i18ndc(root.domain, "Placeholder is a theme name", "Terminal theme: %1", root.followedName)
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: root.theme.textMuted
                        font.family: root.textFamily
                        font.pixelSize: root.captionPx
                    }
                }
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.smallSpacing
            Layout.bottomMargin: Kirigami.Units.smallSpacing / 2
            implicitHeight: root.theme.lineWidth
            color: root.theme.line
        }

        ListView {
            id: list

            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 0
            implicitHeight: contentHeight
            clip: true
            model: choices
            keyNavigationEnabled: true
            highlightMoveDuration: 0
            boundsBehavior: Flickable.StopAtBounds
            section.property: "family"
            section.criteria: ViewSection.FullString
            section.delegate: PlasmaComponents3.Label {
                required property string section

                width: ListView.view.width
                topPadding: Kirigami.Units.smallSpacing
                leftPadding: Kirigami.Units.smallSpacing
                bottomPadding: Kirigami.Units.smallSpacing / 2
                text: root.familyLabel(section)
                textFormat: Text.PlainText
                font.family: root.textFamily
                font.pixelSize: root.captionPx
                font.weight: Font.DemiBold
                font.capitalization: Font.SmallCaps
                color: root.theme.textMuted
                Accessible.role: Accessible.Heading
            }

            // The popover's height settles after the rows are in (the layout caps
            // it), so the current row is brought into view again whenever it changes.
            onHeightChanged: {
                if (root.opened) {
                    positionViewAtIndex(currentIndex, ListView.Contain);
                }
            }

            Keys.onReturnPressed: root.choose(currentIndex)
            Keys.onEnterPressed: root.choose(currentIndex)
            Keys.onSpacePressed: root.choose(currentIndex)
            Keys.onUpPressed: event => {
                if (currentIndex <= 0) {
                    followRow.forceActiveFocus();
                    event.accepted = true;
                }
            }

            PlasmaComponents3.ScrollBar.vertical: PlasmaComponents3.ScrollBar {}

            delegate: QQC2.ItemDelegate {
                id: row

                required property int index
                required property string themeId
                required property string name
                required property string family
                required property string bg
                required property string fg
                required property string accent
                required property string secondary

                readonly property bool active: root.pinned && themeId === root.pinnedId

                width: ListView.view.width
                // The keyboard cursor shows only while the list has the focus.
                highlighted: ListView.isCurrentItem && list.activeFocus
                hoverEnabled: true
                focusPolicy: Qt.NoFocus
                Accessible.role: Accessible.RadioButton
                Accessible.name: i18ndc(root.domain, "Theme name, then its family", "Pin %1, %2 theme",
                                        name, root.familyLabel(family))
                Accessible.checkable: true
                Accessible.checked: active

                onClicked: root.choose(index)

                background: Rectangle {
                    radius: root.theme.radius
                    color: row.highlighted ? root.theme.alpha(root.theme.accent, 0.22)
                        : row.hovered ? root.theme.alpha(root.theme.fg, 0.07) : "transparent"
                }

                contentItem: RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        implicitWidth: Kirigami.Units.iconSizes.small
                        implicitHeight: implicitWidth
                        source: row.active ? "checkmark" : ""
                        isMask: true
                        color: root.theme.accent
                    }
                    ThemeSwatches {
                        colors: [row.bg, row.fg, row.accent, row.secondary]
                        outline: root.theme.line
                        size: Kirigami.Units.smallSpacing * 2
                    }
                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: row.name
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: root.theme.fg
                        font.family: root.textFamily
                        font.pixelSize: root.textPx
                        font.weight: row.active ? Font.DemiBold : Font.Normal
                    }
                }
            }
        }

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            Layout.topMargin: Kirigami.Units.smallSpacing
            visible: !root.listSupported
            text: i18nd(root.domain, "The background service is too old to offer other themes.")
            wrapMode: Text.Wrap
            color: root.theme.textMuted
            font.family: root.textFamily
            font.pixelSize: root.textPx
        }
    }
}
