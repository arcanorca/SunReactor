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
