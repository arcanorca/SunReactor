.pragma library

/// Clock time in the user's locale and hour convention; empty when unknown.
function timeText(epochSeconds) {
    if (!epochSeconds || epochSeconds <= 0) {
        return "";
    }
    return Qt.formatTime(new Date(epochSeconds * 1000));
}
