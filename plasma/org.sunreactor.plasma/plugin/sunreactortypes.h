#pragma once

#include <QtGlobal>

namespace SunReactor {

constexpr quint32 PROTOCOL_VERSION = 1;
constexpr qint64 MAX_IPC_MESSAGE_BYTES = 64 * 1024; // 64 KiB ceiling guard
constexpr int DEFAULT_REQUEST_TIMEOUT_MS = 3000;
/// Poll cadence while the popup is open and the user may be adjusting values.
constexpr int DEFAULT_ACTIVE_POLL_INTERVAL_MS = 1000;
/// Poll cadence while only the panel icon and its tooltip are visible.
constexpr int DEFAULT_IDLE_POLL_INTERVAL_MS = 10000;

/// Sentinel for "the daemon did not report a value", used for every percentage
/// the widget shows. The UI renders it as an em dash, never as a number.
constexpr int UNKNOWN_PERCENT = -1;

} // namespace SunReactor
