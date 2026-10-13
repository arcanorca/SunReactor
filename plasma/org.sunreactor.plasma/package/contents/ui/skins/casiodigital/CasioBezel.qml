/*
 * A watch case around one module: a moulded dark ring with a hairline highlight
 * on its inside top edge, the title silk-screened on the case, and the glass (the
 * liquid crystal) holding the content.
 *
 * Children go on the glass, stacked top to bottom. `footer` takes items on the
 * case itself, under the glass, laid out in a row (the physical keys).
 *
 *   CasioBezel {
 *       style: root.style
 *       title: "Displays"
 *       trailing: "2 CH"
 *       footerShown: true
 *       footer: Repeater { ... CasioKey { ... } }
 *       CasioDigits { ... }      // on the glass
 *   }
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property CasioStyle style
    property string title: ""
    property string trailing: ""
    /*! Items on the glass, stacked top to bottom. */
    default property alias content: glassBody.data
    /*! Items on the case under the glass, in a row. */
    property alias footer: footerRow.data
    property bool footerShown: false

    implicitWidth: caseBody.implicitWidth + 2 * root.style.pad
    implicitHeight: caseBody.implicitHeight + 2 * root.style.pad

    // The case: a dark moulded ring, a shade at the foot, a hairline edge.
    Rectangle {
        anchors.fill: parent
        radius: root.style.caseRadius
        antialiasing: true
        border.width: root.style.hairline
        border.color: root.style.caseEdge
        gradient: Gradient {
            GradientStop { position: 0.0; color: root.style.caseTop }
            GradientStop { position: 1.0; color: root.style.caseBottom }
        }
    }

    // The hairline highlight that makes the edge read as a bevel.
    Rectangle {
        x: root.style.caseRadius
        y: root.style.hairline
        width: root.width - 2 * root.style.caseRadius
        height: root.style.hairline
        color: root.style.caseShine
    }

    ColumnLayout {
        id: caseBody

        x: root.style.pad
        y: root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap

        RowLayout {
            Layout.fillWidth: true
            visible: root.title.length > 0 || root.trailing.length > 0
            spacing: root.style.gap

            Text {
                Layout.fillWidth: true
                text: root.title
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeCap)
                color: root.style.legend
                elide: Text.ElideRight
            }
            Text {
                visible: root.trailing.length > 0
                text: root.trailing
                textFormat: Text.PlainText
                font: root.style.cap(root.style.sizeCap)
                color: root.style.legendMuted
            }
        }

        // The glass: a grey-green panel with a shaded inside edge.
        Rectangle {
            id: glass

            Layout.fillWidth: true
            implicitHeight: glassBody.implicitHeight + 2 * root.style.glassPad
            radius: root.style.glassRadius
            antialiasing: true
            color: root.style.glass
            border.width: root.style.hairline
            border.color: root.style.glassShade

            ColumnLayout {
                id: glassBody

                x: root.style.glassPad
                y: root.style.glassPad
                width: glass.width - 2 * root.style.glassPad
                spacing: root.style.gap
            }
        }

        RowLayout {
            id: footerRow

            Layout.fillWidth: true
            Layout.topMargin: root.style.gap
            visible: root.footerShown
            spacing: root.style.gap
        }
    }
}
