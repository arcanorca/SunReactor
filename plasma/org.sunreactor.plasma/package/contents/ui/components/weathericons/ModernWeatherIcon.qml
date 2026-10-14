/*
 * Modern weather icon: a full-colour symbol in the style of the platform weather
 * sets. A gradient sun with a soft glow, layered white and grey clouds with a
 * blue-grey underside, blue gradient raindrops, white snowflakes, a golden bolt, a
 * pale crescent with a few stars. The colours are fixed, never the theme's, so the
 * icon looks the same in every theme. Only the contrast guard follows the surface
 * behind it: on a light surface each shape gets a rim one device pixel wide (the
 * same path stroked darker, half of it outside the shape), so white clouds, the
 * pale moon and the snow keep their outline. On a dark surface there is no rim.
 *
 *   ModernWeatherIcon { theme: view.theme; condition: "rain"; night: false; size: 22 }
 *
 * Crisp at any size: the box is a whole number of device pixels and the geometry is
 * laid out on that grid. Geometry lives in ModernGeometry.js.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import "ModernGeometry.js" as G

Item {
    id: root

    required property var theme
    required property string condition
    required property bool night
    /*! Logical px, square. */
    property real size: 22
    /*! The colour the icon sits on: the contrast guard reads it. */
    property color surface: theme.bg
    /*! Contract only (the icon's own colours are fixed). */
    property color ink: theme.fg
    /*! True on a dark surface: no rim. Derived from the surface's luminance; a host may set it. */
    property bool onDark: G.luminance(surface) < 0.2

    readonly property real dpr: theme.grid.devicePixelRatio > 0 ? theme.grid.devicePixelRatio : 1
    /*! Side in whole device pixels, and the same side in logical px. */
    readonly property int devSize: Math.max(1, Math.round(size * dpr))
    readonly property real side: devSize / dpr
    /*! Logical px per design-grid unit. */
    readonly property real unit: side / G.GRID
    /*! Two device px: one outside the shape, one under it. */
    readonly property real rimWidth: 2 / dpr
    /*! The layers, in px (ModernGeometry.js). */
    readonly property var g: G.scene(condition, night, unit)

    width: side
    height: side
    implicitWidth: side
    implicitHeight: side

    /*! A fill: one layer's path, with a gradient or a colour. */
    component Fill: ShapePath {
        required property string d

        strokeColor: "transparent"
        fillColor: "transparent"
        // The layers' sub-paths overlap (a cloud is four discs and a base): nonzero unites them.
        fillRule: ShapePath.WindingFill
        PathSvg { path: d }
    }

    /*! The rim of a layer: the same path stroked under the fill, on light surfaces only. */
    component Rim: ShapePath {
        required property string d
        required property color tint

        fillColor: "transparent"
        fillRule: ShapePath.WindingFill
        strokeColor: root.onDark ? "transparent" : tint
        strokeWidth: root.onDark ? 0 : root.rimWidth
        joinStyle: ShapePath.RoundJoin
        PathSvg { path: d }
    }

    /*! Top-to-bottom gradient over a layer's box: the clouds' underside comes from it. */
    component Vertical: LinearGradient {
        required property var box

        x1: box[0]
        y1: box[1]
        x2: box[0]
        y2: box[3]
    }

    Shape {
        anchors.fill: parent
        antialiasing: true
        preferredRendererType: Shape.CurveRenderer

        // Halo behind the sun or the moon.
        Fill {
            d: root.g.glow.d
            fillGradient: RadialGradient {
                centerX: root.g.glow.cx
                centerY: root.g.glow.cy
                focalX: root.g.glow.cx
                focalY: root.g.glow.cy
                centerRadius: root.g.glow.r
                GradientStop { position: 0.0; color: root.night ? Qt.rgba(1, 0.96, 0.8, 0.42) : Qt.rgba(1, 0.84, 0.36, 0.5) }
                GradientStop { position: 0.45; color: root.night ? Qt.rgba(1, 0.96, 0.8, 0.16) : Qt.rgba(1, 0.84, 0.36, 0.2) }
                GradientStop { position: 1.0; color: Qt.rgba(1, 0.84, 0.36, 0) }
            }
        }

        // Back cloud (cloudy), greyer, behind the front cloud.
        Rim { d: root.g.back.d; tint: "#6d8299" }
        Fill {
            d: root.g.back.d
            fillGradient: Vertical {
                box: root.g.back.box
                GradientStop { position: 0.0; color: "#f7fafd" }
                GradientStop { position: 0.7; color: "#cad7e5" }
                GradientStop { position: 1.0; color: "#9fb4cc" }
            }
        }

        // Sun: a disc with rays, lit from the upper left.
        Rim { d: root.g.sun.d; tint: "#b3620e" }
        Fill {
            d: root.g.sun.d
            fillGradient: RadialGradient {
                centerX: root.g.sun.cx
                centerY: root.g.sun.cy
                focalX: root.g.sun.cx - root.g.sun.r * 0.25
                focalY: root.g.sun.cy - root.g.sun.r * 0.25
                centerRadius: root.g.sun.r * 1.15
                GradientStop { position: 0.0; color: "#fff8cf" }
                GradientStop { position: 0.5; color: "#ffd046" }
                GradientStop { position: 0.82; color: "#ffb229" }
                GradientStop { position: 1.0; color: "#f58a1f" }
            }
        }

        // Crescent moon.
        Rim { d: root.g.moon.d; tint: "#a88423" }
        Fill {
            d: root.g.moon.d
            fillGradient: LinearGradient {
                x1: root.g.moon.box[0]
                y1: root.g.moon.box[1]
                x2: root.g.moon.box[2]
                y2: root.g.moon.box[3]
                GradientStop { position: 0.0; color: "#fffef6" }
                GradientStop { position: 0.55; color: "#fbf0c2" }
                GradientStop { position: 1.0; color: "#efd47e" }
            }
        }

        // Stars beside the moon.
        Rim { d: root.g.stars.d; tint: "#a88423" }
        Fill { d: root.g.stars.d; fillColor: "#fff6cc" }

        // Front cloud: white, with a blue-grey underside.
        Rim { d: root.g.front.d; tint: "#6d8299" }
        Fill {
            d: root.g.front.d
            fillGradient: Vertical {
                box: root.g.front.box
                GradientStop { position: 0.0; color: "#ffffff" }
                GradientStop { position: 0.6; color: "#f8fafc" }
                GradientStop { position: 0.85; color: "#d9e4ef" }
                GradientStop { position: 1.0; color: "#aec3da" }
            }
        }

        // Raindrops and the bolt, over the cloud's base.
        Rim { d: root.g.drops.d; tint: "#1d5cb0" }
        Fill {
            d: root.g.drops.d
            fillGradient: Vertical {
                box: root.g.drops.box
                GradientStop { position: 0.0; color: "#d6f1ff" }
                GradientStop { position: 0.55; color: "#5aaeff" }
                GradientStop { position: 1.0; color: "#2c7de3" }
            }
        }

        Rim { d: root.g.bolt.d; tint: "#a25f06" }
        Fill {
            d: root.g.bolt.d
            fillGradient: Vertical {
                box: root.g.bolt.box
                GradientStop { position: 0.0; color: "#fff9b0" }
                GradientStop { position: 0.55; color: "#ffc83a" }
                GradientStop { position: 1.0; color: "#f59a0b" }
            }
        }

        Rim { d: root.g.snow.d; tint: "#7890ac" }
        Fill { d: root.g.snow.d; fillColor: "#ffffff" }

        // Mist lines and the fog bars: a grey that reads on either surface.
        Fill { d: root.g.mist.d; fillColor: root.onDark ? "#c6d4e2" : "#6f8399" }

        // The unknown condition's question mark.
        Fill { d: root.g.question.d; fillColor: "#56667b" }
    }
}
