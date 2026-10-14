/*
 * The glass an emissive display sits behind (VFD, tubes, CRT, scope screen):
 * a recessed fill, a hairline bezel, an inner top shadow for depth, and the
 * flavour's texture - horizontal filaments for a VFD, scanlines for a CRT.
 * On printed panels (Braun, ThinkPad) `glass` is false and the window is
 * only a transparent padding box, so modules can wrap content unconditionally.
 *
 * Content goes into a ColumnLayout; the window reports its implicit height.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts

Item {
    id: root

    required property InstrumentStyle style
    property bool glass: style.emissive
    property real padding: glass ? style.glassPad : 0
    property real spacing: style.rowGap
    default property alias contentData: column.data

    implicitHeight: style.snap(column.implicitHeight + padding * 2)
    implicitWidth: column.implicitWidth + padding * 2

    Rectangle {
        id: bezel

        anchors.fill: parent
        visible: root.glass
        color: root.style.glass
        radius: root.style.radius
        antialiasing: radius > 0
        border.width: root.style.hairline
        border.color: root.style.glassEdge
    }

    // Inner shadow along the top edge: the glass is set into the panel.
    Rectangle {
        visible: root.glass
        x: root.style.radius
        y: root.style.hairline
        width: parent.width - root.style.radius * 2
        height: root.style.snap(root.style.gap * 1.5)
        gradient: Gradient {
            GradientStop { position: 0; color: Qt.rgba(0, 0, 0, 0.35) }
            GradientStop { position: 1; color: Qt.rgba(0, 0, 0, 0) }
        }
    }

    // VFD filaments: a few tungsten wires stretched across the glass.
    Repeater {
        model: root.glass && root.style.isVfd ? 3 : 0

        delegate: Rectangle {
            required property int index
            x: root.style.hairline
            width: root.width - root.style.hairline * 2
            y: root.style.snap(root.height * (index + 1) / 4)
            height: root.style.hairline
            color: root.style.theme.alpha(root.style.ink, 0.045)
        }
    }

    // CRT scanlines: one dark device row in three, only with phosphor glow.
    Repeater {
        model: root.glass && root.style.isCrt && root.style.glow > 0
            ? Math.floor(root.height * root.style.dpr / 3) : 0

        delegate: Rectangle {
            required property int index
            x: root.style.hairline
            width: root.width - root.style.hairline * 2
            y: (index * 3 + 1) / root.style.dpr
            height: root.style.hairline
            color: Qt.rgba(0, 0, 0, 0.22)
        }
    }

    ColumnLayout {
        id: column

        x: root.padding
        y: root.padding
        width: root.width - root.padding * 2
        spacing: root.spacing
    }
}
