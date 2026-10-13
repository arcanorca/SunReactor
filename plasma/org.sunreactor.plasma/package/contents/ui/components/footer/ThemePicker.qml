/*
 * The theme chip's popover: "Same as the terminal app (<theme>)" or pin one
 * of the daemon's themes (list_themes) for this widget. Rows show swatches,
 * name and family; the active choice is checked, a pinned one carries a lock.
 * Keyboard: Up/Down, Return/Space to choose, Escape to close.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.plasma.components as PlasmaComponents3
import org.kde.kirigami as Kirigami
import org.sunreactor.plasma

import ".."
import "../../engine"

QQC2.Popup {
    id: root

    required property PopupFooter footer

    readonly property Theme theme: footer.theme
    readonly property string domain: footer.domain
    readonly property bool listSupported: footer.client.themeRegistry !== SunReactorClient.CapabilityUnsupported

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

    function choose(index) {
        const entry = choices.get(index);
        if (!entry) {
            return;
        }
        if (entry.kind === "follow") {
            footer.followRequested();
        } else {
            footer.pinRequested(entry.themeId);
        }
        close();
    }

    function rebuild() {
        choices.clear();
        choices.append({ kind: "follow", themeId: "", name: footer.followedName, family: "",
                         bg: "", fg: "", accent: "", secondary: "" });
        const order = ["pixel", "instrument", "terminal"];
        const sorted = footer.client.themeList.slice().sort((a, b) =>
            (order.indexOf(a.family) - order.indexOf(b.family)) || String(a.name).localeCompare(String(b.name)));
        for (const t of sorted) {
            choices.append({ kind: "pin", themeId: t.id, name: t.name, family: t.family,
                             bg: t.bg, fg: t.fg, accent: t.accent, secondary: t.secondaryAccent });
        }
        let current = 0;
        for (let i = 0; i < choices.count; ++i) {
            if (footer.pinned && choices.get(i).themeId === footer.pinnedId) {
                current = i;
            }
        }
        list.currentIndex = current;
        list.positionViewAtIndex(current, ListView.Contain);
    }

    width: Math.min(Kirigami.Units.gridUnit * 18, parent ? parent.Window.width - Kirigami.Units.gridUnit : 400)
    height: Math.min(implicitHeight, Kirigami.Units.gridUnit * 22)
    padding: Kirigami.Units.smallSpacing
    modal: false
    focus: true
    closePolicy: QQC2.Popup.CloseOnEscape | QQC2.Popup.CloseOnPressOutsideParent

    onAboutToShow: {
        footer.client.refreshThemeList();
        rebuild();
    }
    onOpened: list.forceActiveFocus()

    Connections {
        target: root.footer.client
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
        spacing: Kirigami.Units.smallSpacing

        // The daemon fell back to another theme: say why, once, at the top.
        RowLayout {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.smallSpacing
            visible: root.footer.client.themeWarning.length > 0
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
                text: root.footer.client.themeWarning
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
                color: root.theme.fg
                font: Kirigami.Theme.smallFont
            }
        }

        ListView {
            id: list

            Layout.fillWidth: true
            Layout.fillHeight: true
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
                visible: section.length > 0
                height: visible ? implicitHeight + Kirigami.Units.smallSpacing : 0
                topPadding: Kirigami.Units.smallSpacing
                leftPadding: Kirigami.Units.smallSpacing
                text: section.length > 0 ? root.familyLabel(section) : ""
                color: root.theme.textMuted
                font: Kirigami.Theme.smallFont
                Accessible.role: Accessible.Heading
            }

            Keys.onReturnPressed: root.choose(currentIndex)
            Keys.onEnterPressed: root.choose(currentIndex)
            Keys.onSpacePressed: root.choose(currentIndex)

            PlasmaComponents3.ScrollBar.vertical: PlasmaComponents3.ScrollBar {}

            delegate: QQC2.ItemDelegate {
                id: row

                required property int index
                required property string kind
                required property string themeId
                required property string name
                required property string family
                required property string bg
                required property string fg
                required property string accent
                required property string secondary

                readonly property bool isFollow: kind === "follow"
                readonly property bool active: isFollow ? !root.footer.pinned
                                                        : root.footer.pinned && themeId === root.footer.pinnedId

                width: ListView.view.width
                highlighted: ListView.isCurrentItem
                checkable: true
                checked: active
                focusPolicy: Qt.NoFocus
                Accessible.role: Accessible.RadioButton
                Accessible.name: isFollow
                    ? i18ndc(root.domain, "Placeholder is the theme name", "Same as the terminal app (%1)", name)
                    : i18ndc(root.domain, "Theme name, then its family", "Pin %1, %2 theme", name, root.familyLabel(family))

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
                        source: row.active ? (row.isFollow ? "checkmark" : "object-locked") : ""
                        isMask: true
                        color: root.theme.accent
                    }
                    ThemeSwatches {
                        visible: !row.isFollow
                        colors: [row.bg, row.fg, row.accent, row.secondary]
                        outline: root.theme.line
                    }
                    PlasmaComponents3.Label {
                        Layout.fillWidth: true
                        text: row.isFollow
                            ? i18ndc(root.domain, "Placeholder is the theme name", "Same as the terminal app (%1)", row.name)
                            : row.name
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: root.theme.fg
                    }
                    PlasmaComponents3.Label {
                        visible: !row.isFollow
                        text: root.familyLabel(row.family)
                        textFormat: Text.PlainText
                        color: root.theme.textMuted
                        font: Kirigami.Theme.smallFont
                    }
                }
            }
        }

        PlasmaComponents3.Label {
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.smallSpacing
            visible: !root.listSupported
            text: i18nd(root.domain, "The background service is too old to offer other themes.")
            wrapMode: Text.Wrap
            color: root.theme.textMuted
            font: Kirigami.Theme.smallFont
        }
    }
}
