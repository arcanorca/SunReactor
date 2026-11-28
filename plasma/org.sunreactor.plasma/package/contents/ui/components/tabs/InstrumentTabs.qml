/*
 * Tab bar, instrument family: the panel legends of the instrument skins, the
 * same tiny caps as the section rules and Tag.qml. The selected page is ink over
 * an accent segment on the bar's hairline, the others are muted. The weather
 * tab's temperature follows its label in the data face. Left / Right move the
 * selection while the bar has focus; the focused selection takes the Tag outline.
 *
 *     BRIGHTNESS   WEATHER 15°C
 *     ━━━━━━━━━━━  ─────────────────────────────
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as QQC2
import org.kde.kirigami as Kirigami

import "../../engine"
import "../../modules/Modules.js" as Modules
import "../../skins/instrument"

FocusScope {
    id: root

    required property Theme theme
    property var tabs: []
    property string current: ""

    signal picked(string id)
    signal lightModeToggled(bool light)

    readonly property string domain: "plasma_applet_org.sunreactor.plasma"
    readonly property InstrumentStyle style: InstrumentStyle {
        theme: root.theme
    }
    readonly property bool isBraun: root.theme.themeId === "braun"
    readonly property bool braunLight: root.theme.braunLightMode || root.theme.luminance(root.theme.bg) > 0.5
    readonly property real padY: style.snap(style.gap)
    readonly property real switchSize: style.snap(Math.max(18, style.unit * 1.15))

    function tr(msg: string): string {
        return i18nd(root.domain, msg); // qmllint disable unqualified
    }

    implicitWidth: tabRow.implicitWidth + (modeSwitch.visible ? modeSwitch.implicitWidth + style.gap2 : 0)
    implicitHeight: Math.max(tabRow.implicitHeight, modeSwitch.visible ? modeSwitch.implicitHeight : 0)
    activeFocusOnTab: tabs.length > 1

    Accessible.role: Accessible.PageTabList

    Keys.onLeftPressed: root.picked(Modules.step(root.current, -1))
    Keys.onRightPressed: root.picked(Modules.step(root.current, 1))

    // The hairline the legends sit on.
    Rectangle {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: root.style.line
        color: root.style.rule
    }

    Row {
        id: tabRow

        spacing: root.style.snap(root.style.gap2 * 1.5)

        Repeater {
            model: root.tabs

            delegate: Item {
                id: tab

                required property var modelData
                readonly property bool selected: modelData.id === root.current
                readonly property bool hovered: mouse.containsMouse

                implicitWidth: legend.implicitWidth
                implicitHeight: legend.implicitHeight + 2 * root.padY

                Accessible.role: Accessible.PageTab
                Accessible.name: modelData.text
                Accessible.description: modelData.hint || ""
                Accessible.checkable: true
                Accessible.checked: selected
                Accessible.onPressAction: root.picked(tab.modelData.id)

                Row {
                    id: legend

                    y: root.padY
                    spacing: root.style.gap

                    Text {
                        text: tab.modelData.text
                        textFormat: Text.PlainText
                        font: root.style.legendFont
                        color: tab.selected || tab.hovered ? root.style.ink : root.style.inkMuted
                        Accessible.ignored: true
                    }

                    Text {
                        visible: text.length > 0
                        text: tab.modelData.hint || ""
                        textFormat: Text.PlainText
                        font: root.style.dataSmallFont
                        color: root.style.inkMuted
                        Accessible.ignored: true
                    }
                }

                // The accent segment over the hairline, under the selected page.
                Rectangle {
                    visible: tab.selected
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: root.style.line2
                    color: root.style.accent
                }

                // Keyboard focus: the Tag outline around the selected page.
                Rectangle {
                    visible: tab.selected && root.activeFocus
                    anchors.fill: parent
                    anchors.margins: -root.style.gap
                    color: "transparent"
                    border.width: root.style.hairline
                    border.color: root.theme.borderActive
                    radius: root.style.tagRadius
                    antialiasing: radius > 0
                }

                MouseArea {
                    id: mouse

                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: root.picked(tab.modelData.id)
                }
            }
        }
    }

    // In-skin Dieter Rams polarity micro-switch (visible for the Braun skin)
    QQC2.AbstractButton {
        id: modeSwitch

        readonly property real glyphSize: root.style.snap(Math.max(10, root.switchSize * 0.52))

        visible: root.isBraun
        anchors.right: parent.right
        anchors.rightMargin: root.style.gap
        anchors.verticalCenter: tabRow.verticalCenter
        implicitWidth: root.switchSize
        implicitHeight: root.switchSize
        padding: 0
        hoverEnabled: true
        focusPolicy: Qt.StrongFocus
        checkable: true
        checked: root.braunLight

        Accessible.role: Accessible.Button
        Accessible.name: root.braunLight
            ? root.tr("Switch Braun skin to Dark mode")
            : root.tr("Switch Braun skin to Light mode")
        Accessible.checkable: true
        Accessible.checked: root.braunLight
        Accessible.onPressAction: toggleMode()

        QQC2.ToolTip.visible: hovered
        QQC2.ToolTip.text: root.braunLight
            ? root.tr("Switch to Matte Black Panel")
            : root.tr("Switch to Warm Cream Panel")
        QQC2.ToolTip.delay: Kirigami.Units.toolTipDelay

        function toggleMode(): void {
            const next = !root.braunLight;
            root.theme.braunLightModeLocal = next;
            root.lightModeToggled(next);
        }

        onClicked: toggleMode()

        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }

        background: Rectangle {
            radius: width / 2
            antialiasing: true
            color: modeSwitch.down
                ? root.style.theme.alpha(root.style.ink, 0.14)
                : (modeSwitch.hovered ? root.style.theme.alpha(root.style.ink, 0.06) : "transparent")
            border.width: modeSwitch.visualFocus ? root.style.line2 : root.style.hairline
            border.color: modeSwitch.visualFocus
                ? root.style.accent
                : (modeSwitch.hovered ? root.style.ink : root.style.ruleStrong)
        }

        contentItem: Item {
            Canvas {
                id: polarityGlyph

                readonly property color tone: modeSwitch.hovered || root.braunLight
                    ? root.style.ink : root.style.inkMuted
                readonly property bool lightMode: root.braunLight
                readonly property real ringWidth: root.style.line

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
