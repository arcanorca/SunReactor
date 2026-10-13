/*
 * Vector weather glyph: one condition drawn as a 24 x 24 design grid scaled to
 * `size` logical px. Plain colours only, so the panel, modules and settings can
 * all use it; theme wiring belongs to the caller.
 *
 *   WeatherGlyph {
 *       condition: "partly_cloudy"   // see WeatherGlyphs.js for the list
 *       night: false
 *       glyphStyle: "line"           // "line" (monoline) or "solid" (two tones)
 *       palette: { fg: "#f2f2f0", text_muted: "#8a8781", accent: "#ff5500",
 *                  secondary_accent: "#f0b01e", warning: "#d97706", bg: "#161616" }
 *       size: 22
 *   }
 *
 * Crispness: the glyph is laid out in whole device pixels (size x devicePixelRatio,
 * rounded), and the stroke is a whole number of device pixels (2 px minimum).
 * Straight strokes are not pixel-aligned; only their width is snapped.
 *
 * No animation and no timers: the Shape is redrawn only when a property changes.
 */

pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Shapes
import "WeatherGlyphs.js" as G

Item {
    id: root

    /*! "clear", "partly_cloudy", "cloudy", "drizzle", "rain", "heavy_rain",
        "thunderstorm", "snow", "mist", "atmospheric", "fog", or "" (unknown). */
    property string condition: ""
    /*! Night variants swap the sun for a crescent moon. */
    property bool night: false
    /*! "line" (monoline stroke, no fills) or "solid" (flat two-tone fills, no outlines). */
    property string glyphStyle: "line"
    /*! { fg, text_muted, accent, secondary_accent, warning, bg } as colour strings. */
    override property var palette: ({})
    /*! Logical px. */
    property real size: 24

    /*! Effective ratio of the window this glyph sits in (1 outside a window). */
    readonly property real dpr: Window.window && Window.window.devicePixelRatio > 0
                                ? Window.window.devicePixelRatio : 1
    /*! Side in whole device pixels, and the same side in logical px. */
    readonly property int devSize: Math.max(1, Math.round(size * dpr))
    readonly property real side: devSize / dpr
    /*! Logical px per design-grid unit. */
    readonly property real unit: side / G.GRID
    /*! Stroke width: a whole number of device pixels, expressed in logical px. */
    readonly property real strokeWidth: G.strokeDevicePx(devSize) / dpr

    readonly property var elements: G.build(condition, night,
                                            glyphStyle === "solid" ? "solid" : "line",
                                            unit, strokeWidth, palette)

    implicitWidth: side
    implicitHeight: side
    width: side
    height: side

    /*! build() emits at most 11 pieces (checked by the warning below). Shape cannot
        take ShapePath delegates from a Repeater, so each slot is a fixed ShapePath;
        an empty slot draws nothing. */
    readonly property int slotCount: 12

    onElementsChanged: {
        if (elements.length > slotCount) {
            console.warn("WeatherGlyph: " + elements.length + " pieces, only " + slotCount + " slots");
        }
    }

    component Piece: ShapePath {
        id: piece

        required property int slot
        readonly property var el: root.elements.length > slot ? root.elements[slot] : null

        strokeColor: piece.el && piece.el.w > 0 ? piece.el.c : "transparent"
        strokeWidth: piece.el ? piece.el.w : 0
        fillColor: piece.el && piece.el.w === 0 ? piece.el.c : "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg { path: piece.el ? piece.el.d : "" }
    }

    Shape {
        anchors.fill: parent
        antialiasing: true
        preferredRendererType: Shape.CurveRenderer

        Piece { slot: 0 }
        Piece { slot: 1 }
        Piece { slot: 2 }
        Piece { slot: 3 }
        Piece { slot: 4 }
        Piece { slot: 5 }
        Piece { slot: 6 }
        Piece { slot: 7 }
        Piece { slot: 8 }
        Piece { slot: 9 }
        Piece { slot: 10 }
        Piece { slot: 11 }
    }
}
