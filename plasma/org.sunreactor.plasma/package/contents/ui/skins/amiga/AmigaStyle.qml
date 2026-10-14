/*
 * The Amiga skin's tokens: the Workbench four-pen palette and its roles. Every
 * colour is a theme token (the amiga.toml palette: blue desk #0055aa, white,
 * black, orange #ff8800, grey #aaaaaa), so a user copy of the theme gets the
 * same roles. Nothing here is a literal colour.
 *
 *   desk   blue backdrop (the popup background)
 *   body   white window body, bevel highlight, the light pen
 *   face   grey gadgets, title bars, the recessed track
 *   ink    black text, outlines and shadows
 *   hot    orange: the lit gadget, the knob, the now marker
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    readonly property color desk: theme.bg
    readonly property color body: theme.fg
    readonly property color face: theme.secondaryAccent
    readonly property color ink: theme.borderInactive
    readonly property color hot: theme.borderActive
    readonly property color dim: theme.textMuted

    /*! Text on the black screen bar (Telemetry). */
    readonly property color barText: body

    /*! Sprite tone index -> colour (tools/generate_weather_icons.py): 0 none,
        1 highlight, 2 light, 3 mid, 4 shadow, 5 warm, 6 warm edge, 7 water,
        8 outline, 9 ray. Workbench icons: white fills, black outlines, orange sun. */
    readonly property var spriteColors: [
        "transparent", body, body, dim, dim, hot, hot, desk, ink, hot
    ]
}
