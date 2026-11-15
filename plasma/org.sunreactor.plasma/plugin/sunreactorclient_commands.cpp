#include "sunreactorclient_private.h"

#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QProcess>
#include <QStandardPaths>
#include <QUrl>

#include <unistd.h>

using namespace SunReactorClientPrivate;

void SunReactorClient::refreshThemeList()
{
    if (themeListWanted()) {
        enqueueRequest(request(kListThemes));
    }
}

bool SunReactorClient::themeListWanted() const
{
    // An older daemon cannot answer, and one request at a time is enough.
    return m_themeRegistry != CapabilityUnsupported && !isQueued(kListThemes);
}

void SunReactorClient::themeListAnswered(bool gotList)
{
    m_themeListStale = false;
    if (!gotList && m_pollingEnabled && m_isConnected && m_themeRegistry != CapabilityUnsupported
        && !m_themeListRetryTimer->isActive()) {
        m_themeListRetryTimer->start();
    }
}

void SunReactorClient::onThemeListRetryTriggered()
{
    if (m_pollingEnabled && m_isConnected && (m_themeList.isEmpty() || m_themeListStale)) {
        refreshThemeList();
    }
}

void SunReactorClient::fetchTheme(const QString &id)
{
    const QString trimmed = id.trimmed();
    if (trimmed.isEmpty()) {
        Q_EMIT themeFetchFailed(trimmed, QStringLiteral("not_found"));
        return;
    }
    if (m_themeRegistry == CapabilityUnsupported) {
        Q_EMIT themeFetchFailed(trimmed, QStringLiteral("unsupported"));
        return;
    }
    if (isQueued(kGetTheme, trimmed)) {
        return;
    }
    QJsonObject req = request(kGetTheme);
    req[QStringLiteral("id")] = trimmed;
    enqueueRequest(req);
}

void SunReactorClient::requestPinnedTheme()
{
    if (m_pinnedThemeId.isEmpty()) {
        return;
    }
    if (m_themeRegistry == CapabilityUnsupported) {
        setPinned({}, QStringLiteral("unsupported"));
        return;
    }
    fetchTheme(m_pinnedThemeId);
}

void SunReactorClient::refreshDayCurve()
{
    if (m_dayCurveSupport == CapabilityUnsupported || isQueued(kDayCurve)) {
        return;
    }
    if (m_nowEpochS > 0) {
        m_dayCurveDate = QDateTime::fromSecsSinceEpoch(m_nowEpochS).date().toString(Qt::ISODate);
    }
    QJsonObject req = request(kDayCurve);
    req[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
    enqueueRequest(req);
}

void SunReactorClient::onSlowTimerTriggered()
{
    if (!m_pollingEnabled || !m_isConnected) {
        return;
    }
    refreshDayCurve();
    requestPinnedTheme();
}

QString SunReactorClient::defaultSocketPath() const
{
    QString runtimeDir = QStandardPaths::writableLocation(QStandardPaths::RuntimeLocation);
    if (runtimeDir.isEmpty()) {
        runtimeDir = QStringLiteral("/run/user/%1").arg(::getuid());
    }
    return runtimeDir + QStringLiteral("/sunreactor/control.sock");
}

QString SunReactorClient::resolvedSocketPath() const
{
    const QString custom = m_customSocketPath.trimmed();
    return custom.isEmpty() ? defaultSocketPath() : custom;
}
QJsonObject SunReactorClient::request(const QString &name) const
{
    QJsonObject req;
    req[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
    req[QStringLiteral("request")] = name;
    return req;
}

void SunReactorClient::queryStatus()
{
    enqueueRequest(request(QStringLiteral("status")));
}

void SunReactorClient::setGlobalOverride(int percent, int durationMinutes)
{
    QJsonObject req = request(QStringLiteral("set_override"));
    req[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
    req[QStringLiteral("percent")] = qBound(0, percent, 100);
    req[QStringLiteral("minutes")] = durationMinutes > 0 ? QJsonValue(durationMinutes)
                                                         : QJsonValue(QJsonValue::Null);
    enqueueRequest(req);
}

void SunReactorClient::clearGlobalOverride()
{
    QJsonObject req = request(QStringLiteral("clear_override"));
    req[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
    req[QStringLiteral("global")] = true;
    enqueueRequest(req);
}

void SunReactorClient::setMonitorOverride(const QString &monitorId, int percent, int durationMinutes)
{
    const QString id = monitorId.trimmed();
    if (id.isEmpty()) {
        setGlobalOverride(percent, durationMinutes);
        return;
    }
    if (!validRequestIdentifier(id)) {
        fail(QStringLiteral("Monitor id exceeds the 1 KiB request limit"));
        return;
    }

    QJsonObject req = request(QStringLiteral("set_override"));
    req[QStringLiteral("monitor_id")] = id;
    req[QStringLiteral("percent")] = qBound(0, percent, 100);
    req[QStringLiteral("minutes")] = durationMinutes > 0 ? QJsonValue(durationMinutes)
                                                         : QJsonValue(QJsonValue::Null);
    enqueueRequest(req);
}

void SunReactorClient::setMonitorLimits(const QString &monitorId, int minPct, int maxPct)
{
    const QString id = monitorId.trimmed();
    if (id.isEmpty()) {
        return;
    }
    if (!validRequestIdentifier(id)) {
        fail(QStringLiteral("Monitor id exceeds the 1 KiB request limit"));
        return;
    }

    const int clampedMin = qBound(0, minPct, 100);
    const int clampedMax = qBound(clampedMin, maxPct, 100);

    QJsonObject req = request(QStringLiteral("set_monitor_limits"));
    req[QStringLiteral("monitor_id")] = id;
    req[QStringLiteral("min_pct")] = clampedMin;
    req[QStringLiteral("max_pct")] = clampedMax;
    enqueueRequest(req);
}

void SunReactorClient::setCloudDimming(int percent)
{
    QJsonObject req = request(QStringLiteral("set_cloud_dimming"));
    req[QStringLiteral("percent")] = qBound(0, percent, 50);

    // A burst can arrive before the socket's connected signal has written the
    // frame. Replace that not-yet-sent frame as well as queued frames so a
    // ten-keypress burst produces one daemon request.
    if (m_isBusy && !m_responseHandled
        && m_inFlightRequest.value(QStringLiteral("request")).toString()
            == QStringLiteral("set_cloud_dimming")
        && m_socket->state() != QLocalSocket::ConnectedState) {
        m_inFlightRequest = req;
        m_inFlightQueuedAtMs = m_elapsed.elapsed();
        return;
    }
    enqueueRequest(req);
}

void SunReactorClient::clearMonitorOverride(const QString &monitorId)
{
    const QString id = monitorId.trimmed();
    if (id.isEmpty()) {
        clearGlobalOverride();
        return;
    }
    if (!validRequestIdentifier(id)) {
        fail(QStringLiteral("Monitor id exceeds the 1 KiB request limit"));
        return;
    }

    QJsonObject req = request(QStringLiteral("clear_override"));
    req[QStringLiteral("monitor_id")] = id;
    req[QStringLiteral("global")] = false;
    enqueueRequest(req);
}

void SunReactorClient::clearExternalHold(const QString &monitorId)
{
    const QString id = monitorId.trimmed();
    QJsonObject req = request(QStringLiteral("clear_external_hold"));
    req[QStringLiteral("monitor")] = id.isEmpty() ? QJsonValue(QJsonValue::Null) : QJsonValue(id);
    enqueueRequest(req);
}

void SunReactorClient::clearAllOverrides()
{
    // The daemon reads an empty monitor id with `global` unset as "drop every
    // override"; `global: true` would leave per-display ones behind, and those
    // are exactly what the sliders in the popup create.
    QJsonObject req = request(QStringLiteral("clear_override"));
    req[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
    req[QStringLiteral("global")] = false;
    enqueueRequest(req);
}

void SunReactorClient::suspend(int minutes)
{
    QJsonObject req = request(QStringLiteral("suspend"));
    req[QStringLiteral("minutes")] = minutes > 0 ? QJsonValue(minutes) : QJsonValue(QJsonValue::Null);
    enqueueRequest(req);
}

void SunReactorClient::resume()
{
    enqueueRequest(request(QStringLiteral("resume")));
}

void SunReactorClient::runOnce(bool force)
{
    QJsonObject req = request(QStringLiteral("run_once"));
    req[QStringLiteral("force")] = force;
    enqueueRequest(req);
}

void SunReactorClient::refreshWeather()
{
    enqueueRequest(request(QStringLiteral("refresh_weather")));
}

void SunReactorClient::reloadConfig()
{
    enqueueRequest(request(QStringLiteral("reload_config")));
}

void SunReactorClient::ping()
{
    enqueueRequest(request(QStringLiteral("ping")));
}

QString SunReactorClient::themesFolder() const
{
    // The daemon's paths::config_dir(): XDG_CONFIG_HOME when set (it must be
    // absolute, otherwise the daemon refuses it too), else $HOME/.config.
    const QString xdg = qEnvironmentVariable("XDG_CONFIG_HOME");
    if (!xdg.isEmpty()) {
        return QDir::isAbsolutePath(xdg) ? QDir::cleanPath(xdg + QStringLiteral("/sunreactor/themes")) : QString();
    }
    const QString home = qEnvironmentVariable("HOME");
    return home.isEmpty() ? QString() : QDir::cleanPath(home + QStringLiteral("/.config/sunreactor/themes"));
}

QString SunReactorClient::ensureThemesFolder() const
{
    const QString path = themesFolder();
    return !path.isEmpty() && QDir().mkpath(path) ? path : QString();
}

QVariantMap SunReactorClient::harnessConfig() const
{
    QVariantMap out;
    if (qEnvironmentVariableIsEmpty("SUNREACTOR_HARNESS")) {
        return out;
    }
    const QStringList items = qEnvironmentVariable("SUNREACTOR_HARNESS_CONFIG").split(QLatin1Char(' '), Qt::SkipEmptyParts);
    for (const QString &item : items) {
        const qsizetype eq = item.indexOf(QLatin1Char('='));
        if (eq <= 0) {
            continue;
        }
        const QString key = item.left(eq).section(QLatin1Char('/'), -1); // "Group/key" -> "key"
        const QString raw = item.mid(eq + 1);
        QJsonParseError error{};
        const QJsonDocument doc = QJsonDocument::fromJson(QByteArray("[") + raw.toUtf8() + QByteArray("]"), &error);
        out.insert(key, error.error == QJsonParseError::NoError && doc.array().size() == 1
                            ? doc.array().first().toVariant()
                            : QVariant(raw));
    }
    return out;
}

bool SunReactorClient::openThemesFolder() const
{
    const QString path = ensureThemesFolder();
    return !path.isEmpty() && QDesktopServices::openUrl(QUrl::fromLocalFile(path));
}

void SunReactorClient::openTerminal()
{
    QString cliPath = QStandardPaths::findExecutable(QStringLiteral("sunreactorctl"));
    if (cliPath.isEmpty()) {
        const QString localBin = QDir::homePath() + QStringLiteral("/.local/bin/sunreactorctl");
        cliPath = QFileInfo::exists(localBin) ? localBin : QStringLiteral("sunreactorctl");
    }

    // xdg-terminal-exec is the desktop-independent entry point; the rest are
    // fallbacks for sessions that do not ship it.
    const QString xdgTerm = QStandardPaths::findExecutable(QStringLiteral("xdg-terminal-exec"));
    if (!xdgTerm.isEmpty()) {
        QProcess::startDetached(xdgTerm, {cliPath, QStringLiteral("tui")});
        return;
    }

    const QStringList fallbacks = {
        QStringLiteral("konsole"),
        QStringLiteral("kitty"),
        QStringLiteral("alacritty"),
        QStringLiteral("foot"),
        QStringLiteral("xterm"),
    };
    for (const QString &terminal : fallbacks) {
        const QString found = QStandardPaths::findExecutable(terminal);
        if (!found.isEmpty()) {
            QProcess::startDetached(found, {QStringLiteral("-e"), cliPath, QStringLiteral("tui")});
            return;
        }
    }

    fail(QStringLiteral("No terminal emulator found to run sunreactorctl tui"));
}


