.pragma library

/*
 * Display state rules shared by every Displays module and skin: what a
 * client.monitors entry's fields mean for the controls drawn from it. Pure
 * functions of the entry, so a binding that calls one tracks what it passes in.
 */

/*! True while the daemon cannot reach the display (it is not answering). */
function unavailable(monitor) {
    return monitor.unreachable === true || monitor.topology === "temporarily_unavailable";
}

/*! True when a control for the display may write: it takes part in automation and answers. */
function controllable(monitor) {
    return monitor.enabled && !unavailable(monitor);
}
