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

/*! Display rows for repeater models: prepends a master "All displays" row when
    multiple monitors are present. */
function rowsWithMaster(monitors, globalPercent, isOverrideActive, masterLabel) {
    var out = [];
    if (monitors.length > 1) {
        out.push({
            master: true,
            logicalId: masterLabel,
            percent: globalPercent,
            hasOverride: isOverrideActive,
            enabled: true,
            unreachable: false
        });
    }
    for (var i = 0; i < monitors.length; ++i) {
        out.push(monitors[i]);
    }
    return out;
}

/*! True when the range/limits drawer for logicalId is expanded in expandedMap. */
function isExpanded(expandedMap, logicalId) {
    return expandedMap !== undefined && expandedMap !== null
        && expandedMap[String(logicalId)] === true;
}

/*! Returns a copy of expandedMap with logicalId's expanded state flipped. */
function toggleExpanded(expandedMap, logicalId) {
    var next = {};
    if (expandedMap) {
        for (var key in expandedMap) {
            if (expandedMap[key] === true) {
                next[key] = true;
            }
        }
    }
    var target = String(logicalId);
    next[target] = !isExpanded(expandedMap, target);
    return next;
}

/*! Remaining seconds of an active external brightness hold, or 0. */
function externalHoldSeconds(monitor) {
    if (!monitor || !monitor.externalHold) {
        return 0;
    }
    var rem = Number(monitor.externalHold.remainingS);
    return rem > 0 ? rem : 0;
}

/*! True while the daemon is holding automatic writes due to an external change. */
function hasExternalHold(monitor) {
    return externalHoldSeconds(monitor) > 0;
}

/*! Compact duration label ("<1m" or "Nm") for external-hold badges. */
function externalHoldAge(seconds) {
    var s = Math.max(0, Number(seconds) || 0);
    return s < 60 ? "<1m" : Math.ceil(s / 60) + "m";
}

/*! True while live or stale cached daemon status is available to display. */
function hasStatus(client) {
    return client !== null && client !== undefined
        && (client.isConnected === true || client.statusStale === true);
}

/*! True while live or stale cached monitor rows are available to display. */
function hasDisplayData(client) {
    return hasStatus(client) && client.monitors !== undefined
        && client.monitors !== null && client.monitors.length > 0;
}

/*! True when both minPct and maxPct are reported (non-negative). */
function hasValidLimits(monitor) {
    return monitor !== null && monitor !== undefined
        && monitor.minPct !== undefined && monitor.maxPct !== undefined
        && Number(monitor.minPct) >= 0 && Number(monitor.maxPct) >= 0;
}

/*! Full status notice for contested or write-deferred displays, or "". */
function monitorNotice(monitor, contestedText, deferredText) {
    if (!monitor) {
        return "";
    }
    if (monitor.contested === true) {
        return contestedText !== undefined
            ? contestedText
            : qsTr("Paused: another app is changing this display");
    }
    if (monitor.writesDeferred === true) {
        return deferredText !== undefined
            ? deferredText
            : qsTr("Queued, applying shortly");
    }
    return "";
}

/*! Short badge tag for compact or slotless skins ("Paused" / "Queued"), or "". */
function monitorTag(monitor, pausedText, queuedText) {
    if (!monitor) {
        return "";
    }
    if (monitor.contested === true) {
        return pausedText !== undefined ? pausedText : qsTr("Paused");
    }
    if (monitor.writesDeferred === true) {
        return queuedText !== undefined ? queuedText : qsTr("Queued");
    }
    return "";
}
