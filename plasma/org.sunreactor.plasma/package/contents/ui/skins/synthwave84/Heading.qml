/*
 * Section heading: Orbitron caps sheared 10 degrees like outrun lettering, with
 * a hairline in cyan that runs to the end of the column.
 *
 *   DAYLIGHT ───────────────────────────────────────
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

RowLayout {
    id: root

    required property SynthStyle style
    property string text: ""
    property color tone: style.pink

    Layout.fillWidth: true
    spacing: root.style.gap2

    Text {
        id: label

        Layout.alignment: Qt.AlignVCenter
        text: root.text
        textFormat: Text.PlainText
        font: root.style.headingFont
        color: root.tone
        // Shear about the centre: the top leans right, the baseline stays put.
        transformOrigin: Item.Center
        transform: Matrix4x4 {
            matrix: Qt.matrix4x4(1, -root.style.shear, 0, 0,
                                 0, 1, 0, 0,
                                 0, 0, 1, 0,
                                 0, 0, 0, 1)
        }
    }

    Rectangle {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        implicitHeight: root.style.hairline
        color: root.style.alpha(root.style.cyan, 0.45)
    }
}
