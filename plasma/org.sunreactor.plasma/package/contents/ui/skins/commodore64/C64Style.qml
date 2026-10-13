/*
 * The Commodore 64 skin's tokens: the 40-column text-mode grid and the colour
 * roles of the Pepto-style palette. Every colour is a theme token (palette
 * entry), never a literal, so the pixel conformance check stays meaningful.
 *
 * Grid: a character cell is 6 art px wide (a 5x7 glyph and one blank column) and
 * 8 art px tall (seven glyph rows and one blank row). A module is about 20 to 36
 * cells across at the usual ratios and art pixel sizes (the tab bar is a few
 * cells narrower); every layout takes its column count from `cols` and shortens
 * its labels to fit. The popup frame is `frameWidth` art px thick; modules keep
 * `margin` art px clear of it.
 */

pragma ComponentBehavior: Bound

import QtQuick

import "../../engine"

QtObject {
    id: root

    required property Theme theme

    /*! Cell width and height, art px. */
    readonly property int cw: 6
    readonly property int ch: 8
    /*! Art px between the popup frame and the content. */
    readonly property int margin: 4
    /*! Art px above and below a module's text. The bottom pad is the larger one:
     *  the screen's bottom edge is rounded down to whole art px, so the last
     *  text row of a module needs more than the frame's band (3 art px) clear. */
    readonly property int padTop: 2
    readonly property int padBottom: 4
    /*! Thickness of the light-blue popup frame, art px. */
    readonly property int frameWidth: 3

    /*! The C64 screen colour (blue). */
    readonly property color screen: theme.bg
    /*! Light blue: text, reverse-video fills, the frame. */
    readonly property color ink: theme.fg
    /*! White: values, the lit key, the banner title. */
    readonly property color hot: theme.accent
    /*! Cyan: the weather cost band. */
    readonly property color cyan: theme.secondaryAccent
    /*! Yellow and green and red: warnings, success, errors. */
    readonly property color yellow: theme.warning
    readonly property color green: theme.success
    readonly property color red: theme.error
    /*! Secondary text and unavailable things. */
    readonly property color quiet: theme.textMuted
    /*! Quiet rules and separators. */
    readonly property color rule: theme.borderInactive
    /*! The popup frame. */
    readonly property color frame: theme.borderActive

    /*! Whole cells that fit between the margins of an art width. */
    function cols(artWidth) {
        return Math.max(1, Math.floor((artWidth - 2 * margin) / cw));
    }

    /*! Left edge (art px) of the centred cell grid. */
    function left(artWidth) {
        return margin + Math.floor((artWidth - 2 * margin - cols(artWidth) * cw) / 2);
    }
}
