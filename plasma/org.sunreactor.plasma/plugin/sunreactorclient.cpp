#include "sunreactorclient.h"

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

    // Defer the first query so QML can still assign socketPath this tick.
    QTimer::singleShot(0, this, &SunReactorClient::queryStatus);
}

SunReactorClient::~SunReactorClient()
{
    m_pollTimer->stop();
    m_watchdogTimer->stop();
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
    }
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

    if (type == QStringLiteral("status")) {
        setConnected(true);
        parseStatus(responseObj.value(QStringLiteral("status")).toObject());
    } else if (type == QStringLiteral("ack") || type == QStringLiteral("run_once")) {
        setConnected(true);
        // A mutating command was accepted; read back the authoritative state
        // before anything else in the queue. A peer that answers a status read
        // with an acknowledgement must not send us round that loop forever.
        const QString sent = m_inFlightRequest.value(QStringLiteral("request")).toString();
        if (sent != QStringLiteral("status")) {
            m_requestQueue.prepend(request(QStringLiteral("status")));
        }
    } else if (type == QStringLiteral("pong")) {
        setConnected(true);
    } else if (type == QStringLiteral("error")) {
        fail(QStringLiteral("%1: %2")
                 .arg(responseObj.value(QStringLiteral("code")).toString(),
                      responseObj.value(QStringLiteral("message")).toString()));
    }
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

    parseWeather(statusObj);
    parseMonitors(statusObj);

    Q_EMIT statusChanged();
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
        const bool enabled = monitor.value(QStringLiteral("enabled")).toBool(true);
        const qint64 backoffUntil = optionalEpoch(monitor, "backoff_until_epoch_s");
        const bool unreachable = backoffUntil > m_nowEpochS;

        QVariantMap map;
        map[QStringLiteral("logicalId")] = monitor.value(QStringLiteral("logical_id")).toString();
        map[QStringLiteral("backend")] = monitor.value(QStringLiteral("backend")).toString();
        map[QStringLiteral("topology")] = monitor.value(QStringLiteral("topology")).toString();
        map[QStringLiteral("enabled")] = enabled;
        map[QStringLiteral("appliedPercent")] = applied;
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
    Q_EMIT statusChanged();
}
