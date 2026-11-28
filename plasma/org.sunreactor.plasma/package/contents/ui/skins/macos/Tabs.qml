/*
 * Tab bar, macos: the popup's two pages as one native segmented control. Same
 * contract as components/tabs (tabs, current, picked): Left and Right move the
 * page while the bar has focus.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../engine"

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)
    signal lightModeToggled(bool light)

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property MacStyle style: MacStyle {
        theme: root.theme
    }

    function tr(msg: string): string {
        return i18nd(root.domain, msg); // qmllint disable unqualified
    }

    implicitWidth: barRow.implicitWidth
    implicitHeight: barRow.implicitHeight

    Accessible.role: Accessible.PageTabList

    RowLayout {
        id: barRow

        anchors.fill: parent
        spacing: root.style.gap

        Segmented {
            id: seg

            Layout.fillWidth: true
            style: root.style
            segments: root.tabs
            current: root.current
            pageTabs: true

            onPicked: id => root.picked(id)
        }

        // In-skin macOS Light / Dark appearance toggle capsule button
        QQC2.AbstractButton {
            id: modeSwitch

            readonly property real glyphSize: root.style.snap(Math.max(11, root.style.segmentHeight * 0.48))

            Layout.alignment: Qt.AlignVCenter
            Layout.rightMargin: Math.round(root.style.gap * 0.8)
            implicitWidth: root.style.segmentHeight
            implicitHeight: root.style.segmentHeight
            padding: 0
            hoverEnabled: true
            focusPolicy: Qt.StrongFocus
            checkable: true
            checked: root.style.isLight

            Accessible.role: Accessible.Button
            Accessible.name: root.style.isLight
                ? root.tr("Switch macOS skin to Dark mode")
                : root.tr("Switch macOS skin to Light mode")
            Accessible.checkable: true
            Accessible.checked: root.style.isLight
            Accessible.onPressAction: toggleMode()

            QQC2.ToolTip.visible: hovered
            QQC2.ToolTip.text: root.style.isLight
                ? root.tr("Switch to Dark Appearance")
                : root.tr("Switch to Light Appearance")
            QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

            function toggleMode(): void {
                const next = !root.style.isLight;
                root.theme.macosLightModeLocal = next;
                root.lightModeToggled(next);
            }

            onClicked: toggleMode()

            HoverHandler {
                cursorShape: Qt.PointingHandCursor
            }

            background: Rectangle {
                radius: height / 2
                antialiasing: true
                color: modeSwitch.down
                    ? root.style.segmentOn
                    : (modeSwitch.hovered ? root.style.chip : root.style.segmentTrack)
                border.width: modeSwitch.visualFocus ? root.style.hairline * 2 : root.style.hairline
                border.color: modeSwitch.visualFocus ? root.style.accent : root.style.materialEdge
            }

            contentItem: Item {
                Canvas {
                    id: polarityGlyph

                    readonly property color tone: modeSwitch.hovered || root.style.isLight
                        ? root.style.ink : root.style.inkSecondary
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
                }
            }
        }
    }
}
