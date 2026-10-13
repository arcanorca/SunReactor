/*
 * A numeric readout in the flavour's display technology:
 *
 *   type     the readout face (light grotesk, Plex, mono) with tabular figures
 *   segment  seven-segment VFD cells with ghost segments
 *   nixie    one smoked-glass tube per numeral, stacked cold cathodes behind
 *
 * `size` is the height of a numeral (cap height) in logical px, the same for
 * every style, so layouts do not depend on the technology. `baselineOffset`
 * is the numerals' bottom edge, for Row/RowLayout baseline alignment.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "Glyphs.js" as Glyphs

Item {
    id: root

    required property InstrumentStyle style
    property string text: ""
    property real size: Math.round(style.sizeReadout * 0.72)
    property color color: style.ink
    property string kind: style.digitStyle
    property bool glowing: style.emissive && style.glow > 0
    /*! For "type": the face to draw with (defaults to the readout face). */
    property font font: style.readoutFont

    readonly property var segment: kind === "segment" ? Glyphs.segment(text, size) : null
    readonly property var tubes: kind === "nixie" ? Glyphs.nixie(text, size) : null

    implicitWidth: {
        switch (kind) {
        case "segment":
            return Math.ceil(segment.width);
        case "nixie":
            return Math.ceil(tubes.width);
        default:
            return typeText.implicitWidth;
        }
    }
    implicitHeight: {
        switch (kind) {
        case "segment":
            return Math.ceil(size);
        case "nixie":
            return Math.ceil(tubes.height);
        default:
            return typeText.implicitHeight;
        }
    }
    baselineOffset: {
        switch (kind) {
        case "segment":
            return size;
        case "nixie":
            return tubes.height - Glyphs.nixieMetrics(size).inset;
        default:
            return typeText.baselineOffset;
        }
    }

    Accessible.role: Accessible.StaticText
    Accessible.name: text

    // --- type ------------------------------------------------------------------
    Glow {
        anchors.fill: typeText
        source: typeText
        strength: root.kind === "type" && root.glowing ? root.style.glow * 0.7 : 0
    }

    Text {
        id: typeText

        visible: root.kind === "type"
        text: root.text
        textFormat: Text.PlainText
        color: root.color
        font.family: root.font.family
        font.styleName: root.font.styleName
        font.weight: root.font.weight
        font.letterSpacing: root.font.letterSpacing
        font.features: { "tnum": 1 }
        // Cap height of the bundled grotesks is ~0.71 em.
        font.pixelSize: Math.max(8, Math.round(root.size / 0.71))
    }

    // --- segment -----------------------------------------------------------------
    Shape {
        anchors.fill: parent
        visible: root.kind === "segment"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: -1
            fillColor: root.style.ghost
            PathSvg { path: root.segment ? root.segment.ghost : "" }
        }
    }

    Glow {
        anchors.fill: litSegments
        source: litSegments
        strength: root.kind === "segment" && root.glowing ? root.style.glow : 0
    }

    Shape {
        id: litSegments

        anchors.fill: parent
        visible: root.kind === "segment"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeWidth: -1
            fillColor: root.color
            PathSvg { path: root.segment ? root.segment.lit : "" }
        }
    }

    // --- nixie -------------------------------------------------------------------
    Repeater {
        model: root.kind === "nixie" ? root.tubes.cells : []

        delegate: Item {
            id: tube

            required property var modelData

            x: modelData.x
            width: modelData.w
            height: root.height

            // Smoked glass envelope with a dome and a side reflection.
            Rectangle {
                anchors.fill: parent
                visible: !tube.modelData.lamp
                radius: width * 0.32
                antialiasing: true
                border.width: root.style.hairline
                border.color: root.style.theme.alpha(root.style.ink, 0.14)
                gradient: Gradient {
                    GradientStop { position: 0.0; color: root.style.theme.alpha(root.style.ink, 0.07) }
                    GradientStop { position: 0.35; color: root.style.theme.alpha(root.style.ink, 0.025) }
                    GradientStop { position: 1.0; color: root.style.theme.alpha(root.style.ink, 0.045) }
                }
            }
            Rectangle {
                visible: !tube.modelData.lamp
                x: parent.width * 0.14
                y: parent.height * 0.12
                width: Math.max(root.style.hairline, root.style.snap(parent.width * 0.05))
                height: parent.height * 0.5
                radius: width / 2
                color: root.style.theme.alpha(root.style.ink, 0.06)
            }
        }
    }

    // Unlit cathodes (all tubes in one shape).
    Shape {
        anchors.fill: parent
        visible: root.kind === "nixie"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.style.ghost
            strokeWidth: Math.max(root.style.hairline, root.size * 0.035)
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg {
                path: root.tubes ? root.tubes.cells.map(c => c.ghost).join(" ") : ""
            }
        }
    }

    Glow {
        anchors.fill: litCathodes
        source: litCathodes
        strength: root.kind === "nixie" && root.glowing ? root.style.glow : 0
    }

    Shape {
        id: litCathodes

        anchors.fill: parent
        visible: root.kind === "nixie"
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.color
            strokeWidth: Math.max(root.style.line, root.size * 0.055)
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathSvg {
                path: root.tubes ? root.tubes.cells.filter(c => !c.lamp).map(c => c.lit).join(" ") : ""
            }
        }

        ShapePath {
            strokeWidth: -1
            fillColor: root.color
            PathSvg {
                path: root.tubes ? root.tubes.cells.filter(c => c.lamp).map(c => c.lit).join(" ") : ""
            }
        }
    }
}
