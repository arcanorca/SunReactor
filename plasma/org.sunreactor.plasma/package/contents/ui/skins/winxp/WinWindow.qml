/*
 * An XP window card: the Luna title band over the beige client area, in a 1 px
 * dark-blue frame with 8 px top corners. The caption is bold white with a drop
 * shadow; minimise, maximise and close are decorative caption buttons. Content
 * goes into the client area, top to bottom.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import QtQuick.Shapes

Item {
    id: root

    required property WinStyle style
    property string title: ""
    /*! The client area's children, in order. */
    default property alias content: body.data

    readonly property real edge: style.line

    implicitHeight: body.y + body.implicitHeight + style.pad + edge

    // Frame, client area and title band.
    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: "transparent"
            fillColor: root.style.frameDeep
            PathSvg {
                path: root.style.framePath(0, 0, root.width, root.height,
                                           root.style.radiusTop, root.style.radiusBottom)
            }
        }
        ShapePath {
            strokeColor: "transparent"
            fillColor: root.style.panel
            PathSvg {
                path: root.style.framePath(root.edge, root.edge, root.width - 2 * root.edge,
                                           root.height - 2 * root.edge, root.style.radiusTop - root.edge,
                                           Math.max(0, root.style.radiusBottom - root.edge))
            }
        }
        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0
                y1: 0
                x2: 0
                y2: root.style.titleH
                GradientStop { position: 0; color: root.style.frame }
                GradientStop { position: 0.5; color: root.style.frameLight }
                GradientStop { position: 1; color: root.style.frame }
            }
            PathSvg {
                path: root.style.framePath(root.edge, root.edge, root.width - 2 * root.edge,
                                           root.style.titleH - root.edge, root.style.radiusTop - root.edge, 0)
            }
        }
    }

    // Caption: bold white over a darker offset copy for the drop shadow.
    readonly property real captionX: edge + style.pad
    readonly property real captionW: Math.max(0, width - captionX - (3 * style.capSize + 2 * style.gap)
                                              - style.pad * 2)

    Text {
        id: captionShadow

        x: root.captionX + 1
        y: root.edge + (root.style.titleH - root.edge - height) / 2 + 1
        width: root.captionW
        text: root.title
        textFormat: Text.PlainText
        font: root.style.boldFont
        color: root.style.frameDeep
        opacity: 0.7
        elide: Text.ElideRight
    }
    Text {
        x: captionShadow.x - 1
        y: captionShadow.y - 1
        width: root.captionW
        text: root.title
        textFormat: Text.PlainText
        font: root.style.boldFont
        color: root.style.white
        elide: Text.ElideRight
    }

    // The three caption buttons, decorative: minimise, maximise, close.
    Row {
        id: buttons

        x: root.width - root.edge - root.style.gap - width
        y: root.edge + (root.style.titleH - root.edge - height) / 2
        spacing: root.style.gap * 0.5

        Repeater {
            model: 3

            delegate: Rectangle {
                id: cap

                required property int index

                width: root.style.capSize
                height: root.style.capSize
                radius: 3
                border.width: root.style.line
                border.color: root.style.white
                gradient: Gradient {
                    GradientStop {
                        position: 0
                        color: cap.index === 2 ? root.style.capRedTop : root.style.capBlueTop
                    }
                    GradientStop {
                        position: 1
                        color: cap.index === 2 ? root.style.danger : root.style.accent
                    }
                }

                Shape {
                    anchors.fill: parent
                    preferredRendererType: Shape.CurveRenderer

                    ShapePath {
                        strokeColor: root.style.white
                        strokeWidth: root.style.line * 2
                        fillColor: "transparent"
                        capStyle: ShapePath.RoundCap
                        joinStyle: ShapePath.RoundJoin
                        PathSvg { path: root.style.captionGlyph(cap.index, cap.width) }
                    }
                }
            }
        }
    }

    // Client area.
    ColumnLayout {
        id: body

        x: root.style.pad
        y: root.style.titleH + root.style.pad
        width: root.width - 2 * root.style.pad
        spacing: root.style.gap
    }
}
