/*
 * Tab bar, Nothing: the pages as round pills, the way Automation sets its mode.
 * The selected page is the filled white pill; the others are hairline outlines.
 *
 *   ( BRIGHTNESS )  ( WEATHER 15°C )
 *
 * Same contract as components/tabs/: `tabs` = [{ id, text, hint }], `current`,
 * and `picked(id)`. Left and Right move between pages, Space or Enter picks one.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../engine"
import "../../modules/Modules.js" as Modules

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)
    signal lightModeToggled(bool light)

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property NothingStyle style: NothingStyle {
        theme: root.theme
    }

    function tr(msg: string): string {
        return i18nd(root.domain, msg); // qmllint disable unqualified
    }

    implicitWidth: row.implicitWidth
    implicitHeight: row.implicitHeight
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    RowLayout {
        id: row

        anchors.fill: parent
        spacing: root.style.gap

        Repeater {
            model: root.tabs

            delegate: Pill {
                id: tab

                required property var modelData

                style: root.style
                radio: true
                text: modelData.hint ? modelData.text + " " + modelData.hint : modelData.text
                accessibleName: modelData.text
                accessibleDescription: modelData.hint || ""
                active: modelData.id === root.current

                Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
                Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

                onClicked: root.picked(tab.modelData.id)
            }
        }

        Item {
            Layout.fillWidth: true
        }

        // In-skin Nothing Light / OLED Dark toggle pill button
        QQC2.AbstractButton {
            id: modeSwitch

            readonly property real glyphSize: root.style.snap(Math.max(10, root.style.pillHeight * 0.44))

            Layout.alignment: Qt.AlignVCenter
            Layout.rightMargin: Math.round(root.style.gap * 0.6)
            implicitWidth: root.style.pillHeight
            implicitHeight: root.style.pillHeight
            padding: 0
            hoverEnabled: true
            focusPolicy: Qt.StrongFocus
            checkable: true
            checked: root.style.isLight

            Accessible.role: Accessible.Button
            Accessible.name: root.style.isLight
                ? root.tr("Switch Nothing skin to Dark mode")
                : root.tr("Switch Nothing skin to Light mode")
            Accessible.checkable: true
            Accessible.checked: root.style.isLight
            Accessible.onPressAction: toggleMode()

            QQC2.ToolTip.visible: hovered
            QQC2.ToolTip.text: root.style.isLight
                ? root.tr("Switch to OLED Dark")
                : root.tr("Switch to Ceramic Light")
            QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

            function toggleMode(): void {
                const next = !root.style.isLight;
                root.theme.nothingLightModeLocal = next;
                root.lightModeToggled(next);
            }

            onClicked: toggleMode()

            HoverHandler {
                cursorShape: Qt.PointingHandCursor
            }

            background: Item {
                Rectangle {
                    anchors.fill: parent
                    radius: height / 2
                    antialiasing: true
                    color: modeSwitch.down
                        ? root.style.theme.alpha(root.style.ink, 0.14)
                        : (modeSwitch.hovered ? root.style.theme.alpha(root.style.ink, 0.06) : "transparent")
                    border.width: root.style.hairline
                    border.color: modeSwitch.hovered || modeSwitch.visualFocus
                        ? root.style.ink
                        : root.style.outline
                }

                Rectangle {
                    visible: modeSwitch.visualFocus
                    anchors.fill: parent
                    anchors.margins: -root.style.gap * 0.5
                    radius: height / 2
                    antialiasing: true
                    color: "transparent"
                    border.width: root.style.hairline
                    border.color: root.style.ink
                }
            }

            contentItem: Item {
                Canvas {
                    id: polarityGlyph

                    readonly property color tone: modeSwitch.hovered || root.style.isLight
                        ? root.style.ink : root.style.muted
                    readonly property bool lightMode: root.style.isLight
                    readonly property real ringWidth: Math.max(root.style.hairline, root.style.snap(1.2))

                    anchors.centerIn: parent
                    width: modeSwitch.glyphSize
                    height: modeSwitch.glyphSize
                    onToneChanged: requestPaint()
                    onLightModeChanged: requestPaint()
                    onWidthChanged: requestPaint()
                    onPaint: {
                        const ctx = getContext("2d");
                        ctx.reset();
                        const c = width / 2;
                        const r = Math.max(1, c - ringWidth / 2);
                        ctx.strokeStyle = tone;
                        ctx.fillStyle = tone;
                        ctx.lineWidth = ringWidth;
                        ctx.beginPath();
                        ctx.arc(c, c, r, 0, Math.PI * 2);
                        ctx.stroke();

                        ctx.beginPath();
                        const ri = Math.max(0.5, r - ringWidth * 0.5);
                        if (lightMode) {
                            ctx.arc(c, c, ri, -Math.PI / 2, Math.PI / 2, false);
                        } else {
                            ctx.arc(c, c, ri, Math.PI / 2, -Math.PI / 2, false);
                        }
                        ctx.closePath();
                        ctx.fill();
                    }

                    // Nothing red micro-dot signal (kept strictly inside button bounds)
                    Rectangle {
                        width: Math.max(2, root.style.snap(2.5))
                        height: width
                        radius: width / 2
                        antialiasing: true
                        anchors.top: parent.top
                        anchors.right: parent.right
                        visible: root.style.isLight || modeSwitch.hovered
                        color: root.style.accent
                    }
                }
            }
        }
    }
}
