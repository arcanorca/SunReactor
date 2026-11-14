#include "sunreactorclient.h"

#include <QDateTime>
#include <QDesktopServices>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonValue>
#include <QProcess>
#include <QStandardPaths>
#include <QUrl>
#include <QVariantMap>

#include <algorithm>
#include <utility>

#include <unistd.h>

#include "themenormalize.h"

using SunReactor::UNKNOWN_PERCENT;

namespace
{

/// Reads an optional unsigned epoch. Missing, null and negative all mean
/// "the daemon has no value for this", which the UI shows as unknown.
qint64 optionalEpoch(const QJsonObject &object, const char *key)
{
    const QJsonValue value = object.value(QLatin1String(key));
    if (value.isNull() || value.isUndefined()) {
        return 0;
    }
    const qint64 epoch = static_cast<qint64>(value.toDouble(0));
    return epoch > 0 ? epoch : 0;
}

/// Reads an optional percentage, clamped to the 0..100 scale the daemon uses.
int optionalPercent(const QJsonObject &object, const char *key)
{
    const QJsonValue value = object.value(QLatin1String(key));
    if (!value.isDouble()) {
        return UNKNOWN_PERCENT;
    }
    return qBound(0, value.toInt(), 100);
}

const QString kStatus = QStringLiteral("status");
const QString kDayCurve = QStringLiteral("day_curve");
const QString kListThemes = QStringLiteral("list_themes");
const QString kGetTheme = QStringLiteral("get_theme");

/// How long to wait before asking for the theme list again after a failed or empty answer.
constexpr int kThemeListRetryInterval = 2000;
constexpr int kMaxRequestQueue = 16;
constexpr int kInitialReconnectBackoffMs = 250;
constexpr int kMaxReconnectBackoffMs = 8000;
constexpr qsizetype kMaxDisplayStringLength = 64;

QString requestMonitorKey(const QJsonObject &request)
{
    const QJsonValue monitor = request.value(QStringLiteral("monitor_id"));
    return monitor.isNull() ? QStringLiteral("<global>") : monitor.toString();
}

QString displayString(const QJsonValue &value)
{
    return value.toString().left(kMaxDisplayStringLength);
}

} // namespace

SunReactorClient::SunReactorClient(QObject *parent)
    : SunReactorClient(QString(), parent)
{
}

SunReactorClient::SunReactorClient(const QString &customSocketPath, QObject *parent)
    : QObject(parent)
    , m_socket(new QLocalSocket(this))
    , m_pollTimer(new QTimer(this))
    , m_watchdogTimer(new QTimer(this))
    , m_slowTimer(new QTimer(this))
    , m_themeListRetryTimer(new QTimer(this))
    , m_reconnectTimer(new QTimer(this))
    , m_customSocketPath(customSocketPath)
{
    m_elapsed.start();
    m_socket->setReadBufferSize(SunReactor::MAX_IPC_MESSAGE_BYTES);
    connect(m_socket, &QLocalSocket::connected, this, &SunReactorClient::onSocketConnected);
    connect(m_socket, &QLocalSocket::readyRead, this, &SunReactorClient::onSocketReadyRead);
    connect(m_socket, &QLocalSocket::disconnected, this, &SunReactorClient::onSocketDisconnected);
    connect(m_socket, &QLocalSocket::errorOccurred, this, &SunReactorClient::onSocketError);

    m_watchdogTimer->setSingleShot(true);
    m_watchdogTimer->setInterval(SunReactor::DEFAULT_REQUEST_TIMEOUT_MS);
    connect(m_watchdogTimer, &QTimer::timeout, this, &SunReactorClient::onRequestTimeout);

    m_pollTimer->setInterval(SunReactor::DEFAULT_IDLE_POLL_INTERVAL_MS);
    connect(m_pollTimer, &QTimer::timeout, this, &SunReactorClient::onPollTimerTriggered);
    m_pollTimer->start();

    m_slowTimer->setInterval(SunReactor::SLOW_REFRESH_INTERVAL_MS);
    connect(m_slowTimer, &QTimer::timeout, this, &SunReactorClient::onSlowTimerTriggered);
    m_slowTimer->start();

    m_themeListRetryTimer->setSingleShot(true);
    m_themeListRetryTimer->setInterval(kThemeListRetryInterval);
    connect(m_themeListRetryTimer, &QTimer::timeout, this, &SunReactorClient::onThemeListRetryTriggered);

    m_reconnectTimer->setSingleShot(true);
    connect(m_reconnectTimer, &QTimer::timeout, this, &SunReactorClient::onReconnectTimerTriggered);

    // Defer the first query so QML can still assign socketPath this tick.
    QTimer::singleShot(0, this, &SunReactorClient::queryStatus);
}

SunReactorClient::~SunReactorClient()
{
    m_pollTimer->stop();
    m_watchdogTimer->stop();
    m_slowTimer->stop();
    m_themeListRetryTimer->stop();
    m_reconnectTimer->stop();
    if (m_socket->isOpen()) {
        m_socket->abort();
    }
}

SunReactorClient::Mode SunReactorClient::mode() const
{
    if (!m_isConnected) {
        return Mode::Offline;
    }
    if (m_isSuspended) {
        return Mode::Paused;
    }
    if (m_isOverrideActive) {
        return Mode::Manual;
    }
    if (m_isDesktopIdleDimmed) {
        return Mode::IdleDimmed;
    }
    return Mode::Automatic;
}

qint64 SunReactorClient::overrideUntilEpochS() const
{
    if (m_globalOverrideUntilEpochS > 0) {
        return m_globalOverrideUntilEpochS;
    }
    return m_perMonitorOverrideUntilEpochS;
}

qint64 SunReactorClient::solarNoonEpochS() const
{
    if (m_sunriseEpochS <= 0 || m_sunsetEpochS <= 0) {
        return 0;
    }
    return m_sunriseEpochS + (m_sunsetEpochS - m_sunriseEpochS) / 2;
}

bool SunReactorClient::isDaylight() const
{
    if (m_hasSolarElevation) {
        return m_solarElevation > 0.0;
    }
    if (m_sunriseEpochS > 0 && m_sunsetEpochS > 0 && m_nowEpochS > 0) {
        return m_nowEpochS >= m_sunriseEpochS && m_nowEpochS < m_sunsetEpochS;
    }
    return true;
}

bool SunReactorClient::weatherIsNight() const
{
    if (m_weatherDayPhase == QLatin1String("night")) {
        return true;
    }
    if (m_weatherDayPhase == QLatin1String("day")) {
        return false;
    }
    return !isDaylight();
}

void SunReactorClient::setSocketPath(const QString &path)
{
    if (m_customSocketPath == path) {
        return;
    }

    m_customSocketPath = path;
    m_requestQueue.clear();
    m_readBuffer.clear();
    m_watchdogTimer->stop();
    m_reconnectTimer->stop();
    if (m_socket->isOpen()) {
        m_socket->abort();
    }
    setBusy(false);
    Q_EMIT socketPathChanged();
    queryStatus();
}

void SunReactorClient::setPopupOpen(bool open)
{
    if (m_popupOpen == open) {
        return;
    }

    m_popupOpen = open;
    Q_EMIT popupOpenChanged();

    m_pollTimer->setInterval(open ? SunReactor::DEFAULT_ACTIVE_POLL_INTERVAL_MS
                                  : SunReactor::DEFAULT_IDLE_POLL_INTERVAL_MS);
    if (open) {
        queryStatus();
        // The popup is where the curve is read: make sure it is today's.
        if (m_isConnected) {
            refreshDayCurve();
            requestPinnedTheme();
            if (m_themeList.isEmpty()) {
                refreshThemeList();
            }
        }
    }
}

void SunReactorClient::setPollingEnabled(bool enabled)
{
    if (m_pollingEnabled == enabled) {
        return;
    }
    m_pollingEnabled = enabled;
    if (!enabled) {
        m_pollTimer->stop();
        m_slowTimer->stop();
        m_themeListRetryTimer->stop();
        m_reconnectTimer->stop();
        m_watchdogTimer->stop();
        m_requestQueue.clear();
        m_readBuffer.clear();
        m_responseHandled = true;
        if (m_socket->isOpen()) {
            m_socket->abort();
        }
        setBusy(false);
    } else {
        m_pollTimer->start();
        m_slowTimer->start();
        queryStatus();
    }
    Q_EMIT pollingEnabledChanged();
}

void SunReactorClient::setPinnedThemeId(const QString &id)
{
    const QString trimmed = id.trimmed();
    if (m_pinnedThemeId == trimmed) {
        return;
    }
    m_pinnedThemeId = trimmed;
    Q_EMIT pinnedThemeIdChanged();
    setPinned({}, trimmed.isEmpty() ? QString() : QStringLiteral("pending"));
    requestPinnedTheme();
}

void SunReactorClient::setPinned(const QVariantMap &theme, const QString &error)
{
    if (m_pinnedTheme == theme && m_pinnedThemeError == error) {
        return;
    }
    m_pinnedTheme = theme;
    m_pinnedThemeError = error;
    Q_EMIT pinnedThemeChanged();
}

void SunReactorClient::setThemeRegistry(Capability capability)
{
    if (m_themeRegistry == capability) {
        return;
    }
    m_themeRegistry = capability;
    Q_EMIT capabilitiesChanged();
}

void SunReactorClient::setDayCurveSupport(Capability capability)
{
    if (m_dayCurveSupport == capability) {
        return;
    }
    m_dayCurveSupport = capability;
    Q_EMIT capabilitiesChanged();
}

bool SunReactorClient::isQueued(const QString &name, const QString &id) const
{
    const auto matches = [&name, &id](const QJsonObject &request) {
        return request.value(QStringLiteral("request")).toString() == name
            && (id.isNull() || request.value(QStringLiteral("id")).toString() == id);
    };
    if (m_isBusy && !m_responseHandled && matches(m_inFlightRequest)) {
        return true;
    }
    for (const QueuedRequest &queued : std::as_const(m_requestQueue)) {
        if (matches(queued.object)) {
            return true;
        }
    }
    return false;
}

bool SunReactorClient::isMutatingRequest(const QJsonObject &request)
{
    const QString name = request.value(QStringLiteral("request")).toString();
    return name != QStringLiteral("status") && name != kDayCurve && name != kListThemes
        && name != kGetTheme && name != QStringLiteral("ping");
}

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

void SunReactorClient::setBusy(bool busy)
{
    if (m_isBusy != busy) {
        m_isBusy = busy;
        Q_EMIT busyChanged();
    }
}

void SunReactorClient::setConnected(bool connected)
{
    if (m_isConnected == connected) {
        return;
    }
    m_isConnected = connected;
    Q_EMIT connectionChanged(connected);
    if (connected) {
        // Whatever answers next may be a different (newer or older) daemon: its theme list is read again.
        m_themeListStale = true;
    } else {
        m_themeListRetryTimer->stop();
        // Whatever answers next may be a different (newer or older) daemon.
        setThemeRegistry(CapabilityUnknown);
        setDayCurveSupport(CapabilityUnknown);
        m_dayCurveDate.clear();
        if (!m_dayCurve.isEmpty()) {
            m_dayCurve.clear();
            Q_EMIT dayCurveChanged();
        }
        clearStatus();
    }
}

void SunReactorClient::fail(const QString &message)
{
    m_lastError = message;
    Q_EMIT errorOccurred(m_lastError);
}

void SunReactorClient::pruneStaleMutatingRequests()
{
    if (m_isConnected || m_requestQueue.isEmpty()) {
        return;
    }
    const qint64 now = m_elapsed.elapsed();
    for (auto it = m_requestQueue.begin(); it != m_requestQueue.end();) {
        if (isMutatingRequest(it->object) && now - it->queuedAtMs > m_lastBackoffWindowMs) {
            it = m_requestQueue.erase(it);
        } else {
            ++it;
        }
    }
}

void SunReactorClient::scheduleReconnect()
{
    const int delay = m_reconnectBackoffMs;
    m_lastBackoffWindowMs = delay;
    m_reconnectTimer->start(delay);
    m_reconnectBackoffMs = std::min(kMaxReconnectBackoffMs, delay * 2);
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

    QJsonObject req = request(QStringLiteral("clear_override"));
    req[QStringLiteral("monitor_id")] = id;
    req[QStringLiteral("global")] = false;
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

void SunReactorClient::enqueueRequest(const QJsonObject &requestObj)
{
    if (!m_pollingEnabled) {
        return;
    }
    const QString name = requestObj.value(QStringLiteral("request")).toString();
    const qint64 now = m_elapsed.elapsed();

    // Status is idempotent: never queue a second one behind a pending query.
    if (name == QStringLiteral("status") && isQueued(name)) {
        return;
    }

    // Slider updates and limit edits are replaceable until they reach the
    // daemon: the latest value for one monitor is the only useful one.
    if (name == QStringLiteral("set_override") || name == QStringLiteral("set_monitor_limits")) {
        const QString monitor = requestMonitorKey(requestObj);
        for (QueuedRequest &queued : m_requestQueue) {
            if (queued.object.value(QStringLiteral("request")).toString() == name
                && requestMonitorKey(queued.object) == monitor) {
                queued.object = requestObj;
                queued.queuedAtMs = now;
                return;
            }
        }
    }

    if (name == QStringLiteral("set_cloud_dimming")) {
        for (QueuedRequest &queued : m_requestQueue) {
            if (queued.object.value(QStringLiteral("request")).toString() == name) {
                queued.object = requestObj;
                queued.queuedAtMs = now;
                return;
            }
        }
    }

    if (m_requestQueue.size() >= kMaxRequestQueue) {
        if (!isMutatingRequest(requestObj)) {
            return;
        }
        // Preserve the newest command when many different monitor ids are
        // being edited while the daemon is unavailable.
        auto oldest = std::find_if(m_requestQueue.cbegin(), m_requestQueue.cend(), [](const QueuedRequest &queued) {
            return isMutatingRequest(queued.object);
        });
        if (oldest == m_requestQueue.cend()) {
            return;
        }
        m_requestQueue.erase(oldest);
    }

    m_requestQueue.enqueue({requestObj, now});

    if (!m_isBusy && !m_processNextScheduled && !m_reconnectTimer->isActive()
        && m_socket->state() == QLocalSocket::UnconnectedState) {
        processNextRequest();
    }
}

void SunReactorClient::prependRequest(const QJsonObject &requestObj)
{
    if (!m_pollingEnabled) {
        return;
    }
    if (m_requestQueue.size() >= kMaxRequestQueue) {
        m_requestQueue.removeLast();
    }
    m_requestQueue.prepend({requestObj, m_elapsed.elapsed()});
}

void SunReactorClient::processNextRequest()
{
    m_processNextScheduled = false;
    if (m_reconnectTimer->isActive() || (m_isBusy && !m_responseHandled)) {
        return;
    }
    pruneStaleMutatingRequests();
    if (m_requestQueue.isEmpty()) {
        m_responseHandled = true;
        setBusy(false);
        return;
    }

    setBusy(true);
    m_responseHandled = false;
    const QueuedRequest queued = m_requestQueue.dequeue();
    m_inFlightRequest = queued.object;
    m_inFlightQueuedAtMs = queued.queuedAtMs;
    m_readBuffer.clear();

    m_socket->abort();
    m_watchdogTimer->start();
    m_socket->connectToServer(resolvedSocketPath());
}

void SunReactorClient::onSocketConnected()
{
    sendInFlightRequest();
}

void SunReactorClient::sendInFlightRequest()
{
    const QByteArray data = QJsonDocument(m_inFlightRequest).toJson(QJsonDocument::Compact) + '\n';
    if (data.size() > SunReactor::MAX_IPC_MESSAGE_BYTES) {
        transportError(QStringLiteral("Outgoing IPC request exceeded 64 KiB ceiling (%1 bytes)").arg(data.size()));
        return;
    }

    m_socket->write(data);
    m_socket->flush();
}

void SunReactorClient::onSocketReadyRead()
{
    if (!m_isBusy || m_responseHandled) {
        return;
    }

    const qsizetype remaining = SunReactor::MAX_IPC_MESSAGE_BYTES - m_readBuffer.size();
    if (remaining <= 0 || m_socket->bytesAvailable() > remaining) {
        transportError(QStringLiteral("Incoming IPC stream exceeded 64 KiB ceiling"));
        return;
    }
    m_readBuffer.append(m_socket->read(remaining));

    const int newlineIndex = m_readBuffer.indexOf('\n');
    if (newlineIndex < 0) {
        return;
    }

    const QByteArray line = m_readBuffer.left(newlineIndex).trimmed();
    m_readBuffer.clear();
    if (line.isEmpty()) {
        transportError(QStringLiteral("Empty response line from daemon"));
        return;
    }

    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(line, &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        transportError(QStringLiteral("Malformed JSON response from daemon: %1").arg(parseError.errorString()));
        return;
    }
    if (handleResponse(doc.object())) {
        completeResponse();
    } else {
        transportError();
    }
}

void SunReactorClient::onSocketDisconnected()
{
    if (!m_isBusy || m_responseHandled) {
        m_readBuffer.clear();
        return;
    }

    // A response that arrived without its trailing newline is still valid.
    if (!m_readBuffer.isEmpty()) {
        const QByteArray line = m_readBuffer.trimmed();
        m_readBuffer.clear();
        if (!line.isEmpty()) {
            QJsonParseError parseError{};
            const QJsonDocument doc = QJsonDocument::fromJson(line, &parseError);
            if (parseError.error == QJsonParseError::NoError && doc.isObject()) {
                if (handleResponse(doc.object())) {
                    completeResponse();
                } else {
                    transportError();
                }
                return;
            }
            transportError(QStringLiteral("Malformed JSON response from daemon: %1").arg(parseError.errorString()));
            return;
        }
    }

    transportError(QStringLiteral("Daemon closed the socket before replying"));
}

void SunReactorClient::onSocketError(QLocalSocket::LocalSocketError socketError)
{
    if (socketError == QLocalSocket::PeerClosedError && m_responseHandled) {
        return;
    }

    transportError(m_socket->errorString());
}

void SunReactorClient::onRequestTimeout()
{
    transportError(QStringLiteral("Request timed out after %1 ms").arg(SunReactor::DEFAULT_REQUEST_TIMEOUT_MS));
}

void SunReactorClient::onReconnectTimerTriggered()
{
    pruneStaleMutatingRequests();
    processNextRequest();
}

void SunReactorClient::onPollTimerTriggered()
{
    if (!m_pollingEnabled) {
        return;
    }
    queryStatus();
}

bool SunReactorClient::handleResponse(const QJsonObject &responseObj)
{
    const bool listAnswer = m_inFlightRequest.value(QStringLiteral("request")).toString() == kListThemes;
    if (!dispatchResponse(responseObj)) {
        return false;
    }
    if (listAnswer) {
        const bool gotList = responseObj.value(QStringLiteral("response")).toString() == QLatin1String("themes")
            && !m_themeList.isEmpty();
        themeListAnswered(gotList);
    }
    return true;
}

bool SunReactorClient::dispatchResponse(const QJsonObject &responseObj)
{
    const quint32 version = static_cast<quint32>(responseObj.value(QStringLiteral("version")).toInt());
    if (version != SunReactor::PROTOCOL_VERSION) {
        fail(QStringLiteral("Unsupported IPC protocol version %1 (expected %2)")
                 .arg(version)
                 .arg(SunReactor::PROTOCOL_VERSION));
        return false;
    }

    const QString type = responseObj.value(QStringLiteral("response")).toString();

    const QString sent = m_inFlightRequest.value(QStringLiteral("request")).toString();
    if (type.isEmpty()) {
        fail(QStringLiteral("Missing response type from daemon"));
        return false;
    }

    if (type == QStringLiteral("status")) {
        setConnected(true);
        parseStatus(responseObj.value(QStringLiteral("status")).toObject());
    } else if (type == QStringLiteral("ack") || type == QStringLiteral("run_once")) {
        setConnected(true);
        // A mutating command was accepted; read back the authoritative state
        // before anything else in the queue. A peer that answers a status read
        // (or any read) with an acknowledgement must not send us round that
        // loop forever.
        const bool wasRead = sent == kStatus || sent == kDayCurve || sent == kListThemes || sent == kGetTheme;
        if (!wasRead) {
            // Commands that move the policy also move today's curve.
            const bool curveMoves = sent == QLatin1String("reload_config")
                || sent == QLatin1String("set_monitor_limits")
                || sent == QLatin1String("set_cloud_dimming")
                || sent == QLatin1String("refresh_weather")
                || sent == QLatin1String("run_once");
            if (curveMoves && m_dayCurveSupport != CapabilityUnsupported && !isQueued(kDayCurve)) {
                QJsonObject curve = request(kDayCurve);
                curve[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
                prependRequest(curve);
            }
            if (sent == QLatin1String("reload_config")) {
                // Theme files may have been edited along with the config.
                if (!m_pinnedThemeId.isEmpty() && m_themeRegistry != CapabilityUnsupported) {
                    QJsonObject pinned = request(kGetTheme);
                    pinned[QStringLiteral("id")] = m_pinnedThemeId;
                    prependRequest(pinned);
                }
                // A user theme file may have been added: the list is read again.
                if (themeListWanted()) {
                    enqueueRequest(request(kListThemes));
                }
            }
            enqueueRequest(request(kStatus));
        }
    } else if (type == QStringLiteral("pong")) {
        setConnected(true);
    } else if (type == QStringLiteral("themes")) {
        setConnected(true);
        setThemeRegistry(CapabilitySupported);
        parseThemeList(responseObj.value(QStringLiteral("themes")).toArray());
    } else if (type == QStringLiteral("theme")) {
        setConnected(true);
        setThemeRegistry(CapabilitySupported);
        const QString id = m_inFlightRequest.value(QStringLiteral("id")).toString();
        QJsonObject boundedTheme = responseObj.value(QStringLiteral("theme")).toObject();
        boundedTheme[QStringLiteral("name")] = displayString(boundedTheme.value(QStringLiteral("name")));
        const QVariantMap theme = SunReactor::ThemeData::normalizeTheme(
            boundedTheme);
        if (theme.isEmpty()) {
            if (id == m_pinnedThemeId) {
                setPinned({}, QStringLiteral("not_found"));
            }
            Q_EMIT themeFetchFailed(id, QStringLiteral("not_found"));
        } else {
            if (!m_pinnedThemeId.isEmpty() && id == m_pinnedThemeId) {
                setPinned(theme, QString());
            }
            Q_EMIT themeFetched(id, theme);
        }
    } else if (type == QStringLiteral("day_curve")) {
        setConnected(true);
        setDayCurveSupport(CapabilitySupported);
        parseDayCurve(responseObj.value(QStringLiteral("day_curve")).toObject());
    } else if (type == QStringLiteral("error")) {
        // The transport answered with a valid protocol envelope even though
        // the requested operation failed; keep connectivity separate from the
        // daemon's capability/error state.
        setConnected(true);
        handleError(responseObj);
    } else {
        fail(QStringLiteral("Unknown response type from daemon: %1").arg(type));
        return false;
    }
    return true;
}

void SunReactorClient::completeResponse()
{
    if (m_responseHandled) {
        return;
    }
    m_responseHandled = true;
    m_watchdogTimer->stop();
    m_readBuffer.clear();
    m_reconnectTimer->stop();
    m_reconnectBackoffMs = kInitialReconnectBackoffMs;
    m_lastBackoffWindowMs = kInitialReconnectBackoffMs;
    // The response line is the complete frame. A daemon that leaves the peer
    // open must not hold the next request behind a disconnected() signal.
    m_socket->abort();
    // Let the abort's disconnected signal drain before arming the next request;
    // that signal belongs to the completed frame and must not settle the new one.
    m_processNextScheduled = true;
    setBusy(false);
    QMetaObject::invokeMethod(this, [this]() { processNextRequest(); }, Qt::QueuedConnection);
}

void SunReactorClient::transportError(const QString &message)
{
    if (m_responseHandled) {
        return;
    }
    m_responseHandled = true;
    m_watchdogTimer->stop();
    m_readBuffer.clear();
    m_socket->abort();
    setConnected(false);
    if (!message.isEmpty()) {
        fail(message);
    }
    if (!m_inFlightRequest.isEmpty()) {
        const qint64 now = m_elapsed.elapsed();
        if (!isMutatingRequest(m_inFlightRequest)
            || now - m_inFlightQueuedAtMs <= m_lastBackoffWindowMs) {
            if (m_requestQueue.size() >= kMaxRequestQueue) {
                m_requestQueue.removeLast();
            }
            m_requestQueue.prepend({m_inFlightRequest, m_inFlightQueuedAtMs});
        }
    }
    setBusy(false);
    scheduleReconnect();
}

void SunReactorClient::handleError(const QJsonObject &responseObj)
{
    const QString sent = m_inFlightRequest.value(QStringLiteral("request")).toString();
    const QString code = responseObj.value(QStringLiteral("code")).toString();
    // An older daemon cannot even decode the newer requests and answers
    // `invalid_request`. That says "not supported", not "offline" and not
    // "broken": nothing else about the client's state changes.
    const bool rejected = code == QLatin1String("invalid_request");

    if (sent == kDayCurve) {
        if (rejected) {
            setDayCurveSupport(CapabilityUnsupported);
        }
        // Either way there is no curve for today: unknown stays unknown.
        if (!m_dayCurve.isEmpty()) {
            m_dayCurve.clear();
            Q_EMIT dayCurveChanged();
        }
        return;
    }
    if (sent == kListThemes) {
        if (rejected) {
            setThemeRegistry(CapabilityUnsupported);
            if (!m_themeList.isEmpty()) {
                m_themeList.clear();
                Q_EMIT themeListChanged();
            }
        }
        return;
    }
    if (sent == kGetTheme) {
        const QString id = m_inFlightRequest.value(QStringLiteral("id")).toString();
        QString reason = QStringLiteral("not_found");
        if (rejected && m_themeRegistry != CapabilitySupported) {
            // Without evidence of a registry, a rejected get_theme means the
            // daemon does not know the request at all.
            setThemeRegistry(CapabilityUnsupported);
            reason = QStringLiteral("unsupported");
        } else if (!rejected) {
            reason = QStringLiteral("offline");
        }
        if (!m_pinnedThemeId.isEmpty() && id == m_pinnedThemeId) {
            setPinned({}, reason);
        }
        Q_EMIT themeFetchFailed(id, reason);
        return;
    }

    fail(QStringLiteral("%1: %2").arg(code, displayString(responseObj.value(QStringLiteral("message")))));
}

void SunReactorClient::parseStatus(const QJsonObject &statusObj)
{
    const QString previousThemeId = m_theme.value(QStringLiteral("id")).toString();
    const QString previousWarning = m_themeWarning;
    const bool previousSuspended = m_isSuspended;
    const bool previousOverride = m_isOverrideActive;
    const bool previousIdleDimmed = m_isDesktopIdleDimmed;
    const bool previousHasElevation = m_hasSolarElevation;
    const double previousElevation = m_solarElevation;
    const qint64 previousNow = m_nowEpochS;
    const qint64 previousSunrise = m_sunriseEpochS;
    const qint64 previousSunset = m_sunsetEpochS;
    const qint64 previousSuspendUntil = m_suspendUntilEpochS;
    const qint64 previousGlobalOverrideUntil = m_globalOverrideUntilEpochS;
    const qint64 previousMonitorOverrideUntil = m_perMonitorOverrideUntilEpochS;
    const int previousGlobalPercent = m_globalPercent;
    const int previousCloudDimming = m_cloudDimmingPercent;
    const bool previousWeatherEnabled = m_weatherEnabled;
    const bool previousWeatherStale = m_weatherStale;
    const bool previousHasWeather = m_hasWeatherReading;
    const QString previousWeatherState = m_weatherState;
    const QString previousWeatherCondition = m_weatherCondition;
    const QString previousWeatherDayPhase = m_weatherDayPhase;
    const int previousCloudCover = m_cloudCoverPercent;
    const bool previousHasTemperature = m_hasTemperature;
    const double previousTemperature = m_temperatureC;
    const double previousWeatherMultiplier = m_weatherMultiplier;
    const QString previousThemeName = m_themeName;
    const QString previousThemeAccent = m_themeAccent;
    const QString previousThemeSecondaryAccent = m_themeSecondaryAccent;
    const QString previousThemeBg = m_themeBg;
    const QString previousThemeFg = m_themeFg;
    const QString previousThemeTextMuted = m_themeTextMuted;
    const bool previousDaylight = isDaylight();
    const bool previousWeatherNight = weatherIsNight();

    m_isSuspended = statusObj.value(QStringLiteral("suspended")).toBool(false);
    m_isOverrideActive = statusObj.value(QStringLiteral("manual_override_active")).toBool(false);
    m_isDesktopIdleDimmed = statusObj.value(QStringLiteral("desktop_idle_dimmed")).toBool(false);

    const QJsonValue elevation = statusObj.value(QStringLiteral("solar_elevation"));
    m_hasSolarElevation = elevation.isDouble();
    m_solarElevation = m_hasSolarElevation ? elevation.toDouble() : 0.0;

    m_nowEpochS = optionalEpoch(statusObj, "now_epoch_s");
    m_sunriseEpochS = optionalEpoch(statusObj, "sunrise_epoch_s");
    m_sunsetEpochS = optionalEpoch(statusObj, "sunset_epoch_s");
    m_suspendUntilEpochS = optionalEpoch(statusObj, "suspend_until_epoch_s");
    m_globalOverrideUntilEpochS = optionalEpoch(statusObj, "global_override_until_epoch_s");
    m_perMonitorOverrideUntilEpochS = optionalEpoch(statusObj, "per_monitor_override_until_epoch_s");

    const QJsonObject themeObj = statusObj.value(QStringLiteral("theme")).toObject();
    m_themeName = displayString(themeObj.value(QStringLiteral("name")));
    m_themeAccent = themeObj.value(QStringLiteral("accent")).toString();
    m_themeSecondaryAccent = themeObj.value(QStringLiteral("secondary_accent")).toString();
    m_themeBg = themeObj.value(QStringLiteral("bg")).toString();
    m_themeFg = themeObj.value(QStringLiteral("fg")).toString();
    m_themeTextMuted = themeObj.value(QStringLiteral("text_muted")).toString();
    // Optional and omitted when there is nothing to say; a non-string is
    // treated as nothing too.
    m_themeWarning = statusObj.value(QStringLiteral("theme_warning")).toString().trimmed();

    parseWeather(statusObj);
    parseMonitors(statusObj);

    if (previousNow != m_nowEpochS) {
        Q_EMIT nowChanged();
    }
    const bool scalarChanged = previousSuspended != m_isSuspended || previousOverride != m_isOverrideActive
        || previousIdleDimmed != m_isDesktopIdleDimmed || previousHasElevation != m_hasSolarElevation
        || previousElevation != m_solarElevation || previousSunrise != m_sunriseEpochS
        || previousSunset != m_sunsetEpochS || previousSuspendUntil != m_suspendUntilEpochS
        || previousGlobalOverrideUntil != m_globalOverrideUntilEpochS
        || previousMonitorOverrideUntil != m_perMonitorOverrideUntilEpochS
        || previousGlobalPercent != m_globalPercent || previousWeatherEnabled != m_weatherEnabled
        || previousWeatherStale != m_weatherStale || previousHasWeather != m_hasWeatherReading
        || previousWeatherState != m_weatherState || previousWeatherCondition != m_weatherCondition
        || previousWeatherDayPhase != m_weatherDayPhase || previousCloudCover != m_cloudCoverPercent
        || previousCloudDimming != m_cloudDimmingPercent
        || previousHasTemperature != m_hasTemperature || previousTemperature != m_temperatureC
        || previousWeatherMultiplier != m_weatherMultiplier || previousThemeName != m_themeName
        || previousThemeAccent != m_themeAccent || previousThemeSecondaryAccent != m_themeSecondaryAccent
        || previousThemeBg != m_themeBg || previousThemeFg != m_themeFg
        || previousThemeTextMuted != m_themeTextMuted || previousWarning != m_themeWarning
        || previousDaylight != isDaylight() || previousWeatherNight != weatherIsNight();
    if (scalarChanged) {
        Q_EMIT statusChanged();
    }

    parseTheme(statusObj);

    // A different followed theme or warning (a user theme was added, fixed or broken):
    // the daemon's registry may have changed, so the list is read again.
    if (m_theme.value(QStringLiteral("id")).toString() != previousThemeId || m_themeWarning != previousWarning) {
        m_themeListStale = true;
    }
    // Queued rather than sent: this runs inside a response, and the disconnect that follows starts it.
    if (m_themeListStale && themeListWanted()) {
        enqueueRequest(request(kListThemes));
    }

    // Slow data: once per local day (and on the triggers documented at
    // refreshDayCurve), not once per poll.
    if (m_nowEpochS > 0 && m_dayCurveSupport != CapabilityUnsupported) {
        const QString today = QDateTime::fromSecsSinceEpoch(m_nowEpochS).date().toString(Qt::ISODate);
        if (today != m_dayCurveDate) {
            refreshDayCurve();
        }
    }
    if (!m_pinnedThemeId.isEmpty() && m_pinnedTheme.isEmpty()
        && (m_pinnedThemeError.isEmpty() || m_pinnedThemeError == QLatin1String("pending")
            || m_pinnedThemeError == QLatin1String("offline"))
        && !isQueued(kGetTheme, m_pinnedThemeId)) {
        requestPinnedTheme();
    }
}

void SunReactorClient::parseTheme(const QJsonObject &statusObj)
{
    const QJsonValue themeValue = statusObj.value(QStringLiteral("theme"));
    if (!themeValue.isObject()) {
        // A daemon without a theme block tells us nothing either way.
        return;
    }
    QJsonObject themeObj = themeValue.toObject();
    themeObj[QStringLiteral("name")] = displayString(themeObj.value(QStringLiteral("name")));
    const bool hasRegistry = themeObj.value(QStringLiteral("palette")).isObject();
    if (hasRegistry && m_themeRegistry == CapabilityUnsupported) {
        // The daemon was upgraded underneath us: ask again for everything new.
        setDayCurveSupport(CapabilityUnknown);
    }
    setThemeRegistry(hasRegistry ? CapabilitySupported : CapabilityUnsupported);
    if (!hasRegistry && !m_pinnedThemeId.isEmpty()) {
        setPinned({}, QStringLiteral("unsupported"));
    }

    const QVariantMap theme = SunReactor::ThemeData::normalizeTheme(themeObj);
    if (theme.isEmpty() || theme == m_theme) {
        return;
    }
    const bool switched = !m_theme.isEmpty()
        && m_theme.value(QStringLiteral("id")) != theme.value(QStringLiteral("id"));
    m_theme = theme;
    Q_EMIT themeChanged();
    if (switched) {
        // A newly saved TUI theme comes with a config reload on the daemon.
        refreshDayCurve();
    }
}

void SunReactorClient::parseThemeList(const QJsonArray &entries)
{
    QVariantList list;
    for (const QJsonValue &entry : entries) {
        QJsonObject bounded = entry.toObject();
        bounded[QStringLiteral("name")] = displayString(bounded.value(QStringLiteral("name")));
        const QVariantMap summary = SunReactor::ThemeData::normalizeSummary(bounded);
        if (!summary.isEmpty()) {
            list.append(summary);
        }
    }
    if (list != m_themeList) {
        m_themeList = list;
        Q_EMIT themeListChanged();
    }
}

void SunReactorClient::parseDayCurve(const QJsonObject &curve)
{
    QVariantMap out;
    const QJsonArray monitors = curve.value(QStringLiteral("monitors")).toArray();
    QVariantList monitorList;
    for (const QJsonValue &entry : monitors) {
        const QJsonObject monitor = entry.toObject();
        const QString id = displayString(monitor.value(QStringLiteral("monitor_id")));
        if (id.isEmpty()) {
            continue;
        }
        QVariantList samples;
        qint64 lastT = 0;
        for (const QJsonValue &s : monitor.value(QStringLiteral("samples")).toArray()) {
            const QJsonObject sample = s.toObject();
            const qint64 t = optionalEpoch(sample, "t");
            const int pct = optionalPercent(sample, "pct");
            // A curve must move forward in time and carry a value at every step.
            if (t <= lastT || pct == UNKNOWN_PERCENT) {
                continue;
            }
            const int noWeather = optionalPercent(sample, "pct_no_weather");
            lastT = t;
            samples.append(QVariantMap{
                {QStringLiteral("t"), t},
                {QStringLiteral("pct"), pct},
                {QStringLiteral("pctNoWeather"), noWeather == UNKNOWN_PERCENT ? pct : noWeather},
            });
        }
        QVariantList milestones;
        for (const QJsonValue &m : monitor.value(QStringLiteral("milestones")).toArray()) {
            const QJsonObject milestone = m.toObject();
            const qint64 t = optionalEpoch(milestone, "t");
            if (t <= 0) {
                continue;
            }
            milestones.append(QVariantMap{
                // Stable machine id (rise_50, ...) for translation; label is the TUI's English text.
                {QStringLiteral("id"), displayString(milestone.value(QStringLiteral("id")))},
                {QStringLiteral("label"), displayString(milestone.value(QStringLiteral("label")))},
                {QStringLiteral("t"), t},
                {QStringLiteral("pct"), optionalPercent(milestone, "pct")},
            });
        }
        const int minPct = optionalPercent(monitor, "min_pct");
        const int maxPct = optionalPercent(monitor, "max_pct");
        monitorList.append(QVariantMap{
            {QStringLiteral("monitorId"), id},
            {QStringLiteral("minPct"), minPct},
            {QStringLiteral("maxPct"), maxPct},
            {QStringLiteral("samples"), samples},
            {QStringLiteral("milestones"), milestones},
        });
    }

    if (!monitorList.isEmpty()) {
        const QJsonObject sun = curve.value(QStringLiteral("sun")).toObject();
        const QJsonValue multiplier = curve.value(QStringLiteral("weather_multiplier"));
        out[QStringLiteral("date")] = displayString(curve.value(QStringLiteral("date")));
        out[QStringLiteral("stepMinutes")] = curve.value(QStringLiteral("step_minutes")).toInt(0);
        out[QStringLiteral("generatedAtEpochS")] = optionalEpoch(curve, "generated_at_epoch_s");
        out[QStringLiteral("weatherMultiplier")] = multiplier.isDouble() ? multiplier.toDouble() : 1.0;
        out[QStringLiteral("sun")] = QVariantMap{
            {QStringLiteral("dawn"), optionalEpoch(sun, "dawn")},
            {QStringLiteral("sunrise"), optionalEpoch(sun, "sunrise")},
            {QStringLiteral("solarNoon"), optionalEpoch(sun, "solar_noon")},
            {QStringLiteral("sunset"), optionalEpoch(sun, "sunset")},
            {QStringLiteral("dusk"), optionalEpoch(sun, "dusk")},
        };
        out[QStringLiteral("monitors")] = monitorList;
    }

    if (out != m_dayCurve) {
        m_dayCurve = out;
        Q_EMIT dayCurveChanged();
    }
}

void SunReactorClient::parseWeather(const QJsonObject &statusObj)
{
    const QJsonValue weatherValue = statusObj.value(QStringLiteral("weather"));
    if (!weatherValue.isObject()) {
        m_weatherEnabled = false;
        m_weatherStale = false;
        m_hasWeatherReading = false;
        m_weatherState.clear();
        m_weatherCondition.clear();
        m_weatherDescription.clear();
        m_weatherDayPhase.clear();
        m_cloudCoverPercent = UNKNOWN_PERCENT;
        m_cloudDimmingPercent = UNKNOWN_PERCENT;
        m_hasTemperature = false;
        m_temperatureC = 0.0;
        m_weatherMultiplier = 1.0;
        parseForecast({});
        parseDetails({});
        return;
    }

    const QJsonObject weather = weatherValue.toObject();
    m_weatherEnabled = weather.value(QStringLiteral("enabled")).toBool(false);
    // `stale` says the reading is old; the last known values stay on screen,
    // because an hour-old temperature is still better than a blank row.
    m_weatherStale = weather.value(QStringLiteral("stale")).toBool(false);
    m_weatherState = displayString(weather.value(QStringLiteral("state")));
    m_weatherCondition = displayString(weather.value(QStringLiteral("condition")));
    m_weatherDescription = displayString(weather.value(QStringLiteral("condition_description")));
    m_weatherDayPhase = displayString(weather.value(QStringLiteral("day_phase")));
    m_cloudCoverPercent = optionalPercent(weather, "cloud_cover_percent");
    const QJsonValue cloudDimming = weather.value(QStringLiteral("cloud_dimming_percent"));
    m_cloudDimmingPercent = cloudDimming.isDouble() ? qBound(0, cloudDimming.toInt(), 50)
                                                    : UNKNOWN_PERCENT;

    const QJsonValue temperature = weather.value(QStringLiteral("temperature"));
    m_hasTemperature = temperature.isDouble();
    m_temperatureC = m_hasTemperature ? temperature.toDouble() : 0.0;

    const QJsonValue multiplier = weather.value(QStringLiteral("multiplier"));
    m_weatherMultiplier = multiplier.isDouble() ? multiplier.toDouble() : 1.0;

    parseForecast(weather.value(QStringLiteral("forecast")).toArray());
    parseDetails(weather.value(QStringLiteral("details")).toObject());

    const bool hasCondition = !m_weatherCondition.isEmpty()
        && m_weatherCondition != QStringLiteral("unknown");
    m_hasWeatherReading = m_weatherEnabled
        && (hasCondition || m_hasTemperature || m_cloudCoverPercent != UNKNOWN_PERCENT);
}

void SunReactorClient::parseMonitors(const QJsonObject &statusObj)
{
    QVariantList monitors;
    int appliedSum = 0;
    int appliedCount = 0;

    const QJsonArray entries = statusObj.value(QStringLiteral("monitors")).toArray();
    for (const QJsonValue &entry : entries) {
        if (!entry.isObject()) {
            continue;
        }
        const QJsonObject monitor = entry.toObject();

        const int applied = optionalPercent(monitor, "last_applied_percent");
        const int overridePercent = optionalPercent(monitor, "override_percent");
        const int minPct = optionalPercent(monitor, "min_pct");
        const int maxPct = optionalPercent(monitor, "max_pct");
        const bool enabled = monitor.value(QStringLiteral("enabled")).toBool(true);
        const qint64 backoffUntil = optionalEpoch(monitor, "backoff_until_epoch_s");
        const bool unreachable = backoffUntil > m_nowEpochS;

        QVariantMap map;
        map[QStringLiteral("logicalId")] = displayString(monitor.value(QStringLiteral("logical_id")));
        map[QStringLiteral("backend")] = displayString(monitor.value(QStringLiteral("backend")));
        map[QStringLiteral("topology")] = displayString(monitor.value(QStringLiteral("topology")));
        map[QStringLiteral("enabled")] = enabled;
        map[QStringLiteral("appliedPercent")] = applied;
        map[QStringLiteral("minPct")] = minPct != UNKNOWN_PERCENT ? minPct : 15;
        map[QStringLiteral("maxPct")] = maxPct != UNKNOWN_PERCENT ? maxPct : 60;
        map[QStringLiteral("hasOverride")] = overridePercent != UNKNOWN_PERCENT;
        map[QStringLiteral("overridePercent")] = overridePercent;
        // What a slider should show: the override the user set, otherwise the
        // value the daemon last wrote, otherwise nothing.
        map[QStringLiteral("percent")] = overridePercent != UNKNOWN_PERCENT ? overridePercent : applied;
        map[QStringLiteral("unreachable")] = unreachable;
        map[QStringLiteral("backoffRemainingS")] = unreachable ? backoffUntil - m_nowEpochS : 0;

        monitors.append(map);

        if (enabled && applied != UNKNOWN_PERCENT) {
            appliedSum += applied;
            ++appliedCount;
        }
    }

    const bool monitorsDiffer = m_monitors != monitors;
    if (monitorsDiffer) {
        m_monitors = monitors;
        Q_EMIT monitorsChanged();
    }

    const int globalOverride = optionalPercent(statusObj, "global_override_percent");
    if (globalOverride != UNKNOWN_PERCENT) {
        m_globalPercent = globalOverride;
    } else if (appliedCount > 0) {
        m_globalPercent = qRound(static_cast<double>(appliedSum) / appliedCount);
    } else {
        m_globalPercent = UNKNOWN_PERCENT;
    }
}

void SunReactorClient::parseForecast(const QJsonArray &entries)
{
    QVariantList forecast;
    for (const QJsonValue &entry : entries) {
        if (!entry.isObject()) {
            continue;
        }
        const QJsonObject point = entry.toObject();
        const qint64 at = optionalEpoch(point, "dt_epoch_s");
        const QJsonValue temperature = point.value(QStringLiteral("temperature"));
        if (at <= 0 || !temperature.isDouble()) {
            // Without a time and a temperature there is nothing worth drawing.
            continue;
        }

        QVariantMap map;
        map[QStringLiteral("epochS")] = at;
        map[QStringLiteral("temperatureC")] = temperature.toDouble();
        map[QStringLiteral("condition")] = displayString(point.value(QStringLiteral("condition")));
        const QString description = displayString(point.value(QStringLiteral("condition_description")));
        if (!description.isEmpty()) {
            map[QStringLiteral("description")] = description;
        }
        map[QStringLiteral("isNight")] =
            point.value(QStringLiteral("day_phase")).toString() == QLatin1String("night");
        map[QStringLiteral("cloudCoverPercent")] = optionalPercent(point, "cloud_cover_percent");
        map[QStringLiteral("precipitationPercent")] = optionalPercent(point, "precipitation_percent");
        forecast.append(map);
    }
    if (m_forecast != forecast) {
        m_forecast = forecast;
        Q_EMIT forecastChanged();
    }
}

void SunReactorClient::parseDetails(const QJsonObject &details)
{
    QVariantMap map;
    const auto copyNumber = [&details, &map](const char *from, const QString &to) {
        const QJsonValue value = details.value(QLatin1String(from));
        if (value.isDouble()) {
            map[to] = value.toDouble();
        }
    };

    copyNumber("feels_like", QStringLiteral("feelsLikeC"));
    copyNumber("humidity_percent", QStringLiteral("humidityPercent"));
    copyNumber("pressure_hpa", QStringLiteral("pressureHpa"));
    copyNumber("wind_speed_mps", QStringLiteral("windSpeedMps"));
    copyNumber("visibility_m", QStringLiteral("visibilityM"));
    copyNumber("precipitation_percent", QStringLiteral("precipitationPercent"));

    const QJsonValue air = details.value(QStringLiteral("air_quality"));
    if (air.isObject()) {
        const QJsonValue index = air.toObject().value(QStringLiteral("us_aqi"));
        if (index.isDouble()) {
            map[QStringLiteral("airQualityIndex")] = index.toInt();
        }
    }

    if (!m_weatherDescription.isEmpty()) {
        map[QStringLiteral("conditionDescription")] = m_weatherDescription;
    }

    if (m_weatherDetails != map) {
        m_weatherDetails = map;
        Q_EMIT detailsChanged();
    }
}

void SunReactorClient::clearStatus()
{
    const bool previousSuspended = m_isSuspended;
    const bool previousOverride = m_isOverrideActive;
    const bool previousIdleDimmed = m_isDesktopIdleDimmed;
    const int previousGlobalPercent = m_globalPercent;
    const int previousCloudDimming = m_cloudDimmingPercent;
    const QVariantList previousMonitors = m_monitors;
    const bool previousHasWeather = m_hasWeatherReading;
    const QVariantList previousForecast = m_forecast;
    const QVariantMap previousDetails = m_weatherDetails;
    const QString previousThemeName = m_themeName;
    const QString previousThemeAccent = m_themeAccent;
    const QString previousThemeSecondaryAccent = m_themeSecondaryAccent;
    const QString previousThemeBg = m_themeBg;
    const QString previousThemeFg = m_themeFg;
    const QString previousThemeTextMuted = m_themeTextMuted;
    const QString previousThemeWarning = m_themeWarning;
    m_isSuspended = false;
    m_isOverrideActive = false;
    m_isDesktopIdleDimmed = false;
    m_globalPercent = UNKNOWN_PERCENT;
    m_cloudDimmingPercent = UNKNOWN_PERCENT;
    m_monitors.clear();
    m_hasWeatherReading = false;
    m_forecast.clear();
    m_weatherDetails.clear();
    m_themeName.clear();
    m_themeAccent.clear();
    m_themeSecondaryAccent.clear();
    m_themeBg.clear();
    m_themeFg.clear();
    m_themeTextMuted.clear();
    m_themeWarning.clear();
    if (previousMonitors != m_monitors) {
        Q_EMIT monitorsChanged();
    }
    if (previousForecast != m_forecast) {
        Q_EMIT forecastChanged();
    }
    if (previousDetails != m_weatherDetails) {
        Q_EMIT detailsChanged();
    }
    if (previousSuspended != m_isSuspended || previousOverride != m_isOverrideActive
        || previousIdleDimmed != m_isDesktopIdleDimmed || previousGlobalPercent != m_globalPercent
        || previousCloudDimming != m_cloudDimmingPercent
        || previousHasWeather != m_hasWeatherReading || previousThemeName != m_themeName
        || previousThemeAccent != m_themeAccent || previousThemeSecondaryAccent != m_themeSecondaryAccent
        || previousThemeBg != m_themeBg || previousThemeFg != m_themeFg
        || previousThemeTextMuted != m_themeTextMuted || previousThemeWarning != m_themeWarning) {
        Q_EMIT statusChanged();
    }
}
