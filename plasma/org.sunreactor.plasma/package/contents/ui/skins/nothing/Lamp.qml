/*
 * One lit dot on the matrix: the live lamp of the status line and the mark of
 * the one call to action. `tone` is the accent for live and for a call to
 * action, grey when the daemon does not answer. Drawn by DotField (one cell).
 */

pragma ComponentBehavior: Bound

import QtQuick
import org.sunreactor.plasma

DotField {
    id: root

    required property color tone

    cols: 1
    rows: 1
    pitch: 9
    dotScale: 0.7
    litColor: root.tone
    accentColor: root.tone

    // A single cell: no property change ever rebuilds it, so it is lit once.
    Component.onCompleted: fill(DotField.Lit)
}
