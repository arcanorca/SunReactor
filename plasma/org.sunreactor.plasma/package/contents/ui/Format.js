.pragma library

/// Clock time in the user's locale and hour convention; empty when unknown.
function timeText(epochSeconds) {
    if (!epochSeconds || epochSeconds <= 0) {
        return "";
    }
    return Qt.formatTime(new Date(epochSeconds * 1000));
}

/// The same clock time in 24 hours ("15:00"), five characters, for narrow columns.
function timeShortText(epochSeconds) {
    if (!epochSeconds || epochSeconds <= 0) {
        return "";
    }
    return Qt.formatTime(new Date(epochSeconds * 1000), "HH:mm");
}

/// Convert a temperature in Celsius to the daemon's active unit.
function tempValue(celsius, isFahrenheit) {
    return isFahrenheit ? (celsius * 9.0 / 5.0 + 32.0) : celsius;
}

/// Rounded integer temperature in the active unit.
function tempRounded(celsius, isFahrenheit) {
    return Math.round(tempValue(celsius, isFahrenheit));
}

/// Unit suffix for the active temperature unit.
function tempUnit(isFahrenheit) {
    return isFahrenheit ? "°F" : "°C";
}

/// Formatted temperature string (e.g. "15°C" or "59°F").
function tempText(celsius, isFahrenheit, decimals) {
    const v = tempValue(celsius, isFahrenheit);
    const n = decimals > 0 ? v.toFixed(decimals) : String(Math.round(v));
    return n + tempUnit(isFahrenheit);
}
