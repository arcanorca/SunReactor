#pragma once

#include "sunreactorclient.h"

#include <QJsonValue>

#include <algorithm>

namespace SunReactorClientPrivate
{

using SunReactor::UNKNOWN_PERCENT;

inline constexpr int kThemeListRetryInterval = 2000;
inline constexpr int kMaxRequestQueue = 16;
inline constexpr int kInitialReconnectBackoffMs = 250;
inline constexpr int kMaxReconnectBackoffMs = 8000;
inline constexpr qsizetype kMaxDisplayStringLength = 64;
inline constexpr qsizetype kMaxRequestIdentifierLength = 1024;
inline constexpr int kMaxReadTransportRetries = 1;
// Extra wait for a connected daemon that is busy applying a previous change.
inline constexpr int kBusyDaemonGraceMs = 9000;
inline constexpr qint64 kMaxOfflineMutatingAgeMs = 30000;
inline constexpr qsizetype kMaxUnixSocketPathBytes = 107;
inline constexpr qsizetype kMaxThemeWarningLength = 512;

inline const QString kStatus = QStringLiteral("status");
inline const QString kDayCurve = QStringLiteral("day_curve");
inline const QString kListThemes = QStringLiteral("list_themes");
inline const QString kGetTheme = QStringLiteral("get_theme");

inline qint64 optionalEpoch(const QJsonObject &object, const char *key)
{
    const QJsonValue value = object.value(QLatin1String(key));
    if (value.isNull() || value.isUndefined()) {
        return 0;
    }
    const qint64 epoch = static_cast<qint64>(value.toDouble(0));
    return epoch > 0 ? epoch : 0;
}

inline int optionalPercent(const QJsonObject &object, const char *key)
{
    const QJsonValue value = object.value(QLatin1String(key));
    if (!value.isDouble()) {
        return UNKNOWN_PERCENT;
    }
    return qBound(0, value.toInt(), 100);
}

inline QString requestMonitorKey(const QJsonObject &request)
{
    const QString key = request.contains(QStringLiteral("monitor"))
        ? QStringLiteral("monitor")
        : QStringLiteral("monitor_id");
    const QJsonValue monitor = request.value(key);
    return monitor.isNull() || monitor.isUndefined() ? QStringLiteral("<global>") : monitor.toString();
}

inline QString displayString(const QJsonValue &value)
{
    return value.toString().left(kMaxDisplayStringLength);
}

inline bool validRequestIdentifier(const QString &id)
{
    return !id.isEmpty() && id.size() <= kMaxRequestIdentifierLength;
}

} // namespace SunReactorClientPrivate
