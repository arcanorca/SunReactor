/*
 * The theme picker: a compact popover under ThemeButton, right-aligned to it.
 * Its first row is "Follow SunReactor", with the terminal app's theme named
 * under it and a check while following. Below it, one row per daemon theme,
 * grouped by family under small captions: a four-chip palette strip, the name,
 * and a check on the applied one. While the list is empty a status row says
 * whether it is still loading or the daemon is too old to offer other themes.
 * The rows follow the list as it arrives, open or closed.
 *
 * It is a toggle that stays open. The button's click opens and closes it; a
 * press outside the popover closes it (CloseOnPressOutsideParent leaves the
 * button out, so the button's own click is the only toggle and cannot close
 * and reopen the list in one go); Escape closes it. Choosing a row or Follow
 * applies the choice at once and keeps the list open, so themes can be flipped
 * through while the widget updates behind it.
 * Keyboard: Up and Down move the cursor (Up from the first theme reaches Follow),
 * Return, Enter or Space apply the cursor row, Escape closes.
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

    objectName: "themePicker"

    /*! The button the popover hangs from (its Popup parent). */
    required property Item anchorItem
    required property Theme theme
    required property SunReactorClient client
    required property string domain
    /*! The theme the daemon is on, named under the follow row. */
    required property string followedName
    required property bool pinned
    required property string pinnedId
    /*! The daemon's list_themes entries, live: the rows follow every change. */
    required property var themes
    /*! Whether the daemon answers, and the theme registry it reported. */
    required property bool connected
    required property int themeRegistry

    signal followRequested()
    signal pinRequested(string id)

    /*! The status row under Follow while there are no theme rows: loading while the
        daemon answers, or unavailable on a daemon too old to list themes. */
    readonly property string themesNotice: {
        if (choices.count > 0) {
            return "";
        }
        if (root.themeRegistry === SunReactorClient.CapabilityUnsupported) {
            return i18nd(root.domain, "Themes unavailable");
        }
        return root.connected ? i18nd(root.domain, "Loading themes…") : "";
    }
    /*! Small text: the theme's body face (its bitmap face is for the tab bar). */
    readonly property string textFamily: theme.available && theme.bodyFont.length > 0
        ? theme.bodyFont : Kirigami.Theme.defaultFont.family
    readonly property int textPx: 11
    readonly property int captionPx: 9
    /*! One row: 1.6 grid units. */
    readonly property real rowHeight: Kirigami.Units.gridUnit * 1.6
    /*! The popover's width: 11.5 grid units, and never wider than the window allows. */
    readonly property real widthCap: Kirigami.Units.gridUnit * 11.5
    /*! Largest popover height: 60% of the window, and never past its bottom edge. */
    property real maxHeight: 0
    /*! The height the content wants, unclipped (the list's full length included). */
    readonly property real naturalHeight: contentLayout.implicitHeight + topPadding + bottomPadding
    /*! The rows last built; they are rebuilt only when a row's content changes. */
    property string choicesKey: ""

    width: 0
    height: Math.min(naturalHeight, maxHeight)
    // The content fills the capped popover, so the list gets the rest and scrolls.
    contentWidth: width - leftPadding - rightPadding
    contentHeight: height - topPadding - bottomPadding
    padding: Kirigami.Units.smallSpacing / 2
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
        keeps it inside the window. The anchor's window position is read afresh
        on every call, so the popover is right after any change: on open, and
        whenever the anchor, its parents or the window move or resize. */
    function place() {
        const win = anchorItem.Window;
        if (!win) {
            return;
        }
        const origin = anchorItem.mapToItem(null, 0, 0);
        const edge = Kirigami.Units.smallSpacing;
        const wide = Math.max(0, Math.min(root.widthCap, win.width - 2 * edge));
        const left = Math.max(edge, Math.min(origin.x + anchorItem.width - wide, win.width - edge - wide));
        const top = origin.y + anchorItem.height + edge;
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

    /*! The rows, in family order. A list that has not changed keeps its rows, so
        a row under the pointer is never replaced by a refresh. The cursor stays on
        the same theme when the list changes under it; a popover that was empty
        scrolls to the applied row once the rows arrive. */
    function rebuild() {
        const order = ["pixel", "instrument", "terminal"];
        const sorted = root.themes.slice().sort((a, b) =>
            (order.indexOf(a.family) - order.indexOf(b.family)) || String(a.name).localeCompare(String(b.name)));
        const key = sorted.map(t => [t.id, t.name, t.family, t.bg, t.fg, t.accent, t.secondaryAccent].join("\n")).join("\n\n");
        if (key === root.choicesKey) {
            return;
        }
        root.choicesKey = key;
        const cursorId = list.currentIndex >= 0 && list.currentIndex < choices.count
            ? choices.get(list.currentIndex).themeId : "";
        const hadRows = choices.count > 0;
        choices.clear();
        for (const t of sorted) {
            choices.append({ themeId: t.id, name: t.name, family: t.family,
                             bg: t.bg, fg: t.fg, accent: t.accent, secondary: t.secondaryAccent });
        }
        let cursor = -1;
        for (let i = 0; i < choices.count && cursorId.length > 0 && cursor < 0; ++i) {
            if (choices.get(i).themeId === cursorId) {
                cursor = i;
            }
        }
        if (cursor >= 0) {
            list.currentIndex = cursor;
        } else {
            resetCursor();
        }
        if (root.opened && !hadRows && choices.count > 0) {
            scrollToCurrent();
        }
    }

    /*! The keyboard cursor starts on the applied theme, or the first one. */
    function resetCursor() {
        let current = 0;
        for (let i = 0; i < choices.count; ++i) {
            if (root.pinned && choices.get(i).themeId === root.pinnedId) {
                current = i;
                break;
            }
        }
        list.currentIndex = choices.count > 0 ? current : -1;
    }

    /*! Brings the cursor row into view once the list has its final height. */
    function scrollToCurrent() {
        Qt.callLater(() => {
            if (list.currentIndex >= 0) {
                list.positionViewAtIndex(list.currentIndex, ListView.Contain);
            }
        });
    }

    /*! The button's click: open when closed, close when open. */
    function toggle() {
        if (root.opened) {
            root.close();
        } else {
            root.open();
        }
    }

    /*! Applies Follow. The list stays open. */
    function chooseFollow() {
        root.followRequested();
    }

    /*! Applies one theme. The list stays open, and the cursor moves to the row. */
    function choose(index) {
        if (index < 0 || index >= choices.count) {
            return;
        }
        list.currentIndex = index;
        root.pinRequested(choices.get(index).themeId);
    }

    onAboutToShow: {
        place();
        rebuild();
        resetCursor();
    }
    onOpened: {
        place();
        scrollToCurrent();
        if (root.pinned) {
            list.forceActiveFocus();
        } else {
            followRow.forceActiveFocus();
        }
    }
    // The list follows the client's theme list whether the popover is open or not:
    // a list that arrives after the click fills the rows in, with no second click.
    onThemesChanged: root.rebuild()

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
            Layout.leftMargin: Kirigami.Units.smallSpacing
            Layout.rightMargin: Kirigami.Units.smallSpacing
            Layout.bottomMargin: Kirigami.Units.smallSpacing / 2
            visible: root.client.themeWarning.length > 0
            spacing: Kirigami.Units.smallSpacing

            Kirigami.Icon {
                Layout.alignment: Qt.AlignTop
                implicitWidth: Kirigami.Units.iconSizes.small
                implicitHeight: Kirigami.Units.iconSizes.small
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

            objectName: "followRow"
            Layout.fillWidth: true
            implicitHeight: root.rowHeight
            topPadding: 0
            bottomPadding: 0
            leftPadding: Kirigami.Units.smallSpacing
            rightPadding: Kirigami.Units.smallSpacing
            focusPolicy: Qt.StrongFocus
            hoverEnabled: true
            Accessible.role: Accessible.RadioButton
            Accessible.name: i18ndc(root.domain, "Placeholder is the theme name", "Follow SunReactor (%1)", root.followedName)
            Accessible.checkable: true
            Accessible.checked: !root.pinned

            Keys.onDownPressed: list.forceActiveFocus()
            Keys.onReturnPressed: root.chooseFollow()
            Keys.onEnterPressed: root.chooseFollow()
            Keys.onSpacePressed: root.chooseFollow()
            onClicked: root.chooseFollow()

            background: Rectangle {
                radius: root.theme.radius
                color: !root.pinned ? root.theme.alpha(root.theme.accent, 0.14)
                    : followRow.hovered ? root.theme.alpha(root.theme.fg, 0.07) : "transparent"
                border.width: followRow.visualFocus ? root.theme.lineWidth : 0
                border.color: root.theme.accent
            }

            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    implicitWidth: Kirigami.Units.iconSizes.small
                    implicitHeight: Kirigami.Units.iconSizes.small
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
            Layout.topMargin: Kirigami.Units.smallSpacing / 2
            Layout.bottomMargin: Kirigami.Units.smallSpacing / 2
            implicitHeight: root.theme.lineWidth
            color: root.theme.line
        }

        // While the list is empty: loading, or unavailable on an old daemon.
        PlasmaComponents3.Label {
            objectName: "themesStatus"

            Layout.fillWidth: true
            Layout.leftMargin: Kirigami.Units.smallSpacing
            Layout.rightMargin: Kirigami.Units.smallSpacing
            Layout.topMargin: Kirigami.Units.smallSpacing / 2
            Layout.bottomMargin: Kirigami.Units.smallSpacing / 2
            visible: root.themesNotice.length > 0
            text: root.themesNotice
            textFormat: Text.PlainText
            elide: Text.ElideRight
            color: root.theme.textMuted
            font.family: root.textFamily
            font.pixelSize: root.textPx
        }

        ListView {
            id: list

            objectName: "themeList"
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
                topPadding: Kirigami.Units.smallSpacing / 2
                bottomPadding: 0
                leftPadding: Kirigami.Units.smallSpacing
                text: root.familyLabel(section)
                textFormat: Text.PlainText
                font.family: root.textFamily
                font.pixelSize: root.captionPx
                font.weight: Font.DemiBold
                font.capitalization: Font.SmallCaps
                color: root.theme.textMuted
                Accessible.role: Accessible.Heading
            }

            // The height settles when the popover opens and when it is placed:
            // the cursor row is brought back into view whenever it does.
            onHeightChanged: {
                if (root.opened && currentIndex >= 0) {
                    positionViewAtIndex(currentIndex, ListView.Contain);
                }
            }

            Keys.onReturnPressed: root.choose(currentIndex)
            Keys.onEnterPressed: root.choose(currentIndex)
            Keys.onSpacePressed: root.choose(currentIndex)
            // Explicit steps: a handled key does not reach the view's own navigation.
            Keys.onUpPressed: event => {
                if (currentIndex <= 0) {
                    followRow.forceActiveFocus();
                } else {
                    decrementCurrentIndex();
                }
                event.accepted = true;
            }
            Keys.onDownPressed: event => {
                incrementCurrentIndex();
                event.accepted = true;
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

                /*! The applied theme: the pinned one. */
                readonly property bool active: root.pinned && themeId === root.pinnedId
                /*! The keyboard cursor, while the list has the focus. */
                readonly property bool cursor: ListView.isCurrentItem && list.activeFocus

                width: ListView.view.width
                implicitHeight: root.rowHeight
                topPadding: 0
                bottomPadding: 0
                leftPadding: Kirigami.Units.smallSpacing
                rightPadding: Kirigami.Units.smallSpacing
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
                    color: row.active ? root.theme.alpha(root.theme.accent, 0.14)
                        : row.hovered ? root.theme.alpha(root.theme.fg, 0.07) : "transparent"
                    border.width: row.cursor ? root.theme.lineWidth : 0
                    border.color: root.theme.accent
                }

                contentItem: RowLayout {
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        implicitWidth: Kirigami.Units.iconSizes.small
                        implicitHeight: Kirigami.Units.iconSizes.small
                        source: row.active ? "checkmark" : ""
                        isMask: true
                        color: root.theme.accent
                    }
                    // The palette: four chips, 4 x 8 px each.
                    Row {
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 0

                        Repeater {
                            model: [row.bg, row.fg, row.accent, row.secondary]
                            delegate: Rectangle {
                                required property string modelData

                                width: 4
                                height: 8
                                color: modelData
                                border.width: root.theme.hairline
                                border.color: root.theme.line
                            }
                        }
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
    }
}
