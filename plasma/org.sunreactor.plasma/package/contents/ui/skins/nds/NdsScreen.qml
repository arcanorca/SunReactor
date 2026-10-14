/*
 * One of the console's two screens: a thin black bezel with a glossy glass.
 * The top screen (wallpaper "sky") carries the light-blue system wallpaper with
 * a few translucent bubbles and a hinge strip under it; the touch screen
 * (wallpaper "touch") is a pale glass. A diagonal white gloss at 7% sits over
 * everything on the glass. Children are placed in the glass, in glass coordinates.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes

import "../../engine"

Item {
    id: root

    required property Theme theme
    required property NdsStyle style
    /*! "sky" (top screen) or "touch" (bottom screen). */
    property string wallpaper: "touch"
    /*! Height of the glass, logical px. */
    property real glassHeight: 100
    /*! Draw the hinge strip under the bezel (the top screen). */
    property bool hinge: false

    /*! Width of the glass, logical px. */
    readonly property real glassWidth: root.width - 2 * root.style.bezel

    /*! Children are placed in the glass. */
    default property alias content: stage.data

    implicitHeight: bezelBox.height + (root.hinge ? root.style.hingeHeight : 0)

    /*! Fixed bubbles of the system wallpaper: fractions of the glass, radius in px. */
    readonly property var bubbles: [
        { fx: 0.09, fy: 0.14, r: 14 },
        { fx: 0.84, fy: 0.10, r: 21 },
        { fx: 0.95, fy: 0.58, r: 9 },
        { fx: 0.05, fy: 0.80, r: 11 },
        { fx: 0.66, fy: 0.88, r: 17 },
        { fx: 0.74, fy: 0.34, r: 6 }
    ]

    Rectangle {
        id: bezelBox

        width: root.width
        height: root.glassHeight + 2 * root.style.bezel
        radius: root.style.radius
        gradient: Gradient {
            GradientStop { position: 0; color: root.style.bezelTop }
            GradientStop { position: 1; color: root.style.bezelBottom }
        }
    }

    Item {
        id: glass

        x: root.style.bezel
        y: root.style.bezel
        width: root.glassWidth
        height: root.glassHeight
        clip: true

        Rectangle {
            anchors.fill: parent
            gradient: Gradient {
                GradientStop {
                    position: 0
                    color: root.wallpaper === "sky" ? root.style.skyTop : root.style.touchTop
                }
                GradientStop {
                    position: 0.5
                    color: root.wallpaper === "sky" ? root.style.skyMid : root.style.touchMid
                }
                GradientStop {
                    position: 1
                    color: root.wallpaper === "sky" ? root.style.skyBottom : root.style.touchBottom
                }
            }
        }

        Repeater {
            model: root.wallpaper === "sky" ? root.bubbles : []

            delegate: Rectangle {
                required property var modelData

                // Kept whole inside the glass: the clip would cut a bubble at the edge.
                x: Math.max(0, Math.min(glass.width - 2 * modelData.r,
                                        Math.round(modelData.fx * glass.width) - modelData.r))
                y: Math.max(0, Math.min(glass.height - 2 * modelData.r,
                                        Math.round(modelData.fy * glass.height) - modelData.r))
                width: 2 * modelData.r
                height: 2 * modelData.r
                radius: modelData.r
                color: Qt.rgba(1, 1, 1, 0.2)
                border.width: 1
                border.color: Qt.rgba(1, 1, 1, 0.45)
            }
        }

        Item {
            id: stage

            anchors.fill: parent
        }

        // The gloss: a diagonal white band from the top-left corner, 7% at its edge.
        Shape {
            anchors.fill: parent

            ShapePath {
                strokeColor: "transparent"
                strokeWidth: 0
                fillGradient: LinearGradient {
                    x1: 0
                    y1: 0
                    x2: glass.width * 0.7
                    y2: glass.height * 0.7

                    GradientStop { position: 0; color: Qt.rgba(1, 1, 1, 0.07) }
                    GradientStop { position: 0.55; color: Qt.rgba(1, 1, 1, 0) }
                    GradientStop { position: 1; color: Qt.rgba(1, 1, 1, 0) }
                }

                PathPolyline {
                    path: [
                        Qt.point(0, 0),
                        Qt.point(glass.width * 0.9, 0),
                        Qt.point(0, glass.height * 0.9)
                    ]
                }
            }
        }
    }

    // The hinge: a pearl bar under the top screen, with its dark slot in the middle.
    Item {
        visible: root.hinge
        x: 0
        y: bezelBox.height
        width: root.width
        height: root.style.hingeHeight

        Rectangle {
            x: root.style.bezel * 2
            y: 0
            width: root.width - 4 * root.style.bezel
            height: parent.height
            radius: height / 2
            gradient: Gradient {
                GradientStop { position: 0; color: root.style.hingeTop }
                GradientStop { position: 1; color: root.style.hingeBottom }
            }
            border.width: 1
            border.color: root.style.line
        }
        Rectangle {
            x: Math.round(root.width / 2) - 16
            y: Math.round(parent.height / 4)
            width: 32
            height: Math.round(parent.height / 2)
            radius: height / 2
            color: root.style.hingeSlot
        }
    }
}
