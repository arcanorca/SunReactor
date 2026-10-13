#include "sunreactorclient.h"

#include <QDateTime>
#include <QDir>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonValue>
#include <QProcess>
#include <QStandardPaths>
#include <QVariantMap>

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
    , m_customSocketPath(customSocketPath)
{
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

    // Defer the first query so QML can still assign socketPath this tick.
    QTimer::singleShot(0, this, &SunReactorClient::queryStatus);
}

SunReactorClient::~SunReactorClient()
{
    m_pollTimer->stop();
    m_watchdogTimer->stop();
    m_slowTimer->stop();
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
        }
    }
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
    if (m_isBusy && matches(m_inFlightRequest)) {
        return true;
    }
    for (const QJsonObject &queued : std::as_const(m_requestQueue)) {
        if (matches(queued)) {
            return true;
        }
    }
    return false;
}

void SunReactorClient::refreshThemeList()
{
    if (m_themeRegistry == CapabilityUnsupported || isQueued(kListThemes)) {
        return;
    }
    enqueueRequest(request(kListThemes));
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
    if (!m_isConnected) {
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
    if (!connected) {
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
    const QString name = requestObj.value(QStringLiteral("request")).toString();

    // Status is idempotent: never queue a second one behind a pending query.
    if (name == QStringLiteral("status")) {
        for (const QJsonObject &queued : std::as_const(m_requestQueue)) {
            if (queued.value(QStringLiteral("request")).toString() == QStringLiteral("status")) {
                return;
            }
        }
    }

    m_requestQueue.enqueue(requestObj);

    if (m_socket->state() == QLocalSocket::UnconnectedState) {
        processNextRequest();
    }
}

void SunReactorClient::processNextRequest()
{
    if (m_requestQueue.isEmpty()) {
        setBusy(false);
        return;
    }

    setBusy(true);
    m_inFlightRequest = m_requestQueue.dequeue();
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
        m_watchdogTimer->stop();
        m_socket->abort();
        fail(QStringLiteral("Outgoing IPC request exceeded 64 KiB ceiling (%1 bytes)").arg(data.size()));
        processNextRequest();
        return;
    }

    m_socket->write(data);
    m_socket->flush();
}

void SunReactorClient::onSocketReadyRead()
{
    m_readBuffer.append(m_socket->readAll());

    if (m_readBuffer.size() > SunReactor::MAX_IPC_MESSAGE_BYTES) {
        m_watchdogTimer->stop();
        m_socket->abort();
        fail(QStringLiteral("Incoming IPC stream exceeded 64 KiB ceiling (%1 bytes)").arg(m_readBuffer.size()));
        m_readBuffer.clear();
        processNextRequest();
        return;
    }

    int newlineIndex = -1;
    while ((newlineIndex = m_readBuffer.indexOf('\n')) != -1) {
        const QByteArray line = m_readBuffer.left(newlineIndex).trimmed();
        m_readBuffer.remove(0, newlineIndex + 1);

        if (line.isEmpty()) {
            continue;
        }

        m_watchdogTimer->stop();

        QJsonParseError parseError{};
        const QJsonDocument doc = QJsonDocument::fromJson(line, &parseError);
        if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
            fail(QStringLiteral("Malformed JSON response from daemon: %1").arg(parseError.errorString()));
        } else {
            handleResponse(doc.object());
        }
    }
}

void SunReactorClient::onSocketDisconnected()
{
    m_watchdogTimer->stop();

    // A response that arrived without its trailing newline is still valid.
    if (!m_readBuffer.isEmpty()) {
        const QByteArray line = m_readBuffer.trimmed();
        m_readBuffer.clear();
        if (!line.isEmpty()) {
            QJsonParseError parseError{};
            const QJsonDocument doc = QJsonDocument::fromJson(line, &parseError);
            if (parseError.error == QJsonParseError::NoError && doc.isObject()) {
                handleResponse(doc.object());
            }
        }
    }

    processNextRequest();
}

void SunReactorClient::onSocketError(QLocalSocket::LocalSocketError socketError)
{
    if (socketError == QLocalSocket::PeerClosedError) {
        // Expected: the daemon closes the connection after each response.
        return;
    }

    m_watchdogTimer->stop();
    setConnected(false);
    fail(m_socket->errorString());

    m_socket->abort();
    m_readBuffer.clear();
    processNextRequest();
}

void SunReactorClient::onRequestTimeout()
{
    m_socket->abort();
    setConnected(false);
    fail(QStringLiteral("Request timed out after %1 ms").arg(SunReactor::DEFAULT_REQUEST_TIMEOUT_MS));

    m_readBuffer.clear();
    processNextRequest();
}

void SunReactorClient::onPollTimerTriggered()
{
    queryStatus();
}

void SunReactorClient::handleResponse(const QJsonObject &responseObj)
{
    const quint32 version = static_cast<quint32>(responseObj.value(QStringLiteral("version")).toInt());
    if (version != SunReactor::PROTOCOL_VERSION) {
        fail(QStringLiteral("Unsupported IPC protocol version %1 (expected %2)")
                 .arg(version)
                 .arg(SunReactor::PROTOCOL_VERSION));
        return;
    }

    const QString type = responseObj.value(QStringLiteral("response")).toString();

    const QString sent = m_inFlightRequest.value(QStringLiteral("request")).toString();

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
                || sent == QLatin1String("set_monitor_limits") || sent == QLatin1String("refresh_weather")
                || sent == QLatin1String("run_once");
            if (curveMoves && m_dayCurveSupport != CapabilityUnsupported && !isQueued(kDayCurve)) {
                QJsonObject curve = request(kDayCurve);
                curve[QStringLiteral("monitor_id")] = QJsonValue(QJsonValue::Null);
                m_requestQueue.prepend(curve);
            }
            if (sent == QLatin1String("reload_config")) {
                // Theme files may have been edited along with the config.
                if (!m_pinnedThemeId.isEmpty() && m_themeRegistry != CapabilityUnsupported) {
                    QJsonObject pinned = request(kGetTheme);
                    pinned[QStringLiteral("id")] = m_pinnedThemeId;
                    m_requestQueue.prepend(pinned);
                }
            }
            m_requestQueue.prepend(request(kStatus));
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
        const QVariantMap theme = SunReactor::ThemeData::normalizeTheme(
            responseObj.value(QStringLiteral("theme")).toObject());
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
        handleError(responseObj);
    }
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

    fail(QStringLiteral("%1: %2").arg(code, responseObj.value(QStringLiteral("message")).toString()));
}

void SunReactorClient::parseStatus(const QJsonObject &statusObj)
{
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
    m_themeName = themeObj.value(QStringLiteral("name")).toString();
    m_themeAccent = themeObj.value(QStringLiteral("accent")).toString();
    m_themeSecondaryAccent = themeObj.value(QStringLiteral("secondary_accent")).toString();
    m_themeBg = themeObj.value(QStringLiteral("bg")).toString();
    m_themeFg = themeObj.value(QStringLiteral("fg")).toString();
    m_themeTextMuted = themeObj.value(QStringLiteral("text_muted")).toString();

    parseWeather(statusObj);
    parseMonitors(statusObj);

    Q_EMIT statusChanged();

    parseTheme(statusObj);

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
    const QJsonObject themeObj = themeValue.toObject();
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
        const QVariantMap summary = SunReactor::ThemeData::normalizeSummary(entry.toObject());
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
        const QString id = monitor.value(QStringLiteral("monitor_id")).toString();
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
                {QStringLiteral("id"), milestone.value(QStringLiteral("id")).toString()},
                {QStringLiteral("label"), milestone.value(QStringLiteral("label")).toString()},
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
        out[QStringLiteral("date")] = curve.value(QStringLiteral("date")).toString();
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
        m_weatherDayPhase.clear();
        m_cloudCoverPercent = UNKNOWN_PERCENT;
        m_hasTemperature = false;
        m_temperatureC = 0.0;
        m_weatherMultiplier = 1.0;
        m_forecast.clear();
        m_weatherDetails.clear();
        return;
    }

    const QJsonObject weather = weatherValue.toObject();
    m_weatherEnabled = weather.value(QStringLiteral("enabled")).toBool(false);
    // `stale` says the reading is old; the last known values stay on screen,
    // because an hour-old temperature is still better than a blank row.
    m_weatherStale = weather.value(QStringLiteral("stale")).toBool(false);
    m_weatherState = weather.value(QStringLiteral("state")).toString();
    m_weatherCondition = weather.value(QStringLiteral("condition")).toString();
    m_weatherDayPhase = weather.value(QStringLiteral("day_phase")).toString();
    m_cloudCoverPercent = optionalPercent(weather, "cloud_cover_percent");

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
        map[QStringLiteral("logicalId")] = monitor.value(QStringLiteral("logical_id")).toString();
        map[QStringLiteral("backend")] = monitor.value(QStringLiteral("backend")).toString();
        map[QStringLiteral("topology")] = monitor.value(QStringLiteral("topology")).toString();
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

    m_monitors = monitors;

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
        map[QStringLiteral("condition")] = point.value(QStringLiteral("condition")).toString();
        map[QStringLiteral("isNight")] =
            point.value(QStringLiteral("day_phase")).toString() == QLatin1String("night");
        map[QStringLiteral("cloudCoverPercent")] = optionalPercent(point, "cloud_cover_percent");
        map[QStringLiteral("precipitationPercent")] = optionalPercent(point, "precipitation_percent");
        forecast.append(map);
    }
    m_forecast = forecast;
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

    m_weatherDetails = map;
}

void SunReactorClient::clearStatus()
{
    m_isSuspended = false;
    m_isOverrideActive = false;
    m_isDesktopIdleDimmed = false;
    m_globalPercent = UNKNOWN_PERCENT;
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
    Q_EMIT statusChanged();
}
