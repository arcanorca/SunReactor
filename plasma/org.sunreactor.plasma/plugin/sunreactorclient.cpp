#include "sunreactorclient.h"

#include "sunreactorclient_private.h"

using namespace SunReactorClientPrivate;

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
    , m_graceTimer(new QTimer(this))
    , m_customSocketPath(customSocketPath)
{
    m_elapsed.start();
    m_socket->setReadBufferSize(SunReactor::MAX_IPC_MESSAGE_BYTES + 1);
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

    m_graceTimer->setSingleShot(true);
    m_graceTimer->setInterval(kStatusGraceMs);
    connect(m_graceTimer, &QTimer::timeout, this, &SunReactorClient::onGraceTimerTriggered);

    // Defer the first query so QML can still assign socketPath this tick.
    QTimer::singleShot(0, this, &SunReactorClient::queryStatus);
}

SunReactorClient::~SunReactorClient()
{
    m_responseHandled = true;
    m_pollTimer->stop();
    m_watchdogTimer->stop();
    m_slowTimer->stop();
    m_themeListRetryTimer->stop();
    m_reconnectTimer->stop();
    m_graceTimer->stop();
    disconnect(m_socket, nullptr, this, nullptr);
    if (m_socket->isOpen() || m_socket->state() != QLocalSocket::UnconnectedState) {
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
    // A future forecast bucket carries that bucket's own day_phase (for example
    // 09:00 AM tagged "day" while it is still 06:12 AM before sunrise), so
    // live night/day must come from the current solar state when known.
    const bool hasSolarPhase = m_hasSolarElevation
        || (m_sunriseEpochS > 0 && m_sunsetEpochS > 0 && m_nowEpochS > 0);
    if (m_weatherSourceKind == QLatin1String("forecast") && hasSolarPhase) {
        return !isDaylight();
    }
    if (m_hasSolarElevation && m_solarElevation < -6.0) {
        return true;
    }
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
    m_responseHandled = true;
    m_inFlightRequest = QJsonObject{};
    m_inFlightAttempts = 0;
    m_inFlightQueuedAtMs = 0;
    m_watchdogExtended = false;
    m_processNextScheduled = false;
    m_reconnectBackoffMs = kInitialReconnectBackoffMs;
    m_lastBackoffWindowMs = kInitialReconnectBackoffMs;
    m_disconnectedAtMs = -1;
    m_requestQueue.clear();
    m_readBuffer.clear();
    m_watchdogTimer->stop();
    m_reconnectTimer->stop();
    if (m_socket->isOpen() || m_socket->state() != QLocalSocket::UnconnectedState) {
        m_socket->abort();
    }
    setErrorKind(ErrorKind::None);
    setConnected(false, false);
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
        m_responseHandled = true;
        m_inFlightRequest = QJsonObject{};
        m_inFlightAttempts = 0;
        m_inFlightQueuedAtMs = 0;
        m_watchdogExtended = false;
        m_processNextScheduled = false;
        m_reconnectBackoffMs = kInitialReconnectBackoffMs;
        m_lastBackoffWindowMs = kInitialReconnectBackoffMs;
        m_requestQueue.clear();
        m_readBuffer.clear();
        if (m_socket->isOpen() || m_socket->state() != QLocalSocket::UnconnectedState) {
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
    // Keep the previously resolved pinned theme while fetching a new one so
    // switching between two pinned themes does not briefly fall back to the
    // daemon's followed theme.
    setPinned(trimmed.isEmpty() ? QVariantMap{} : m_pinnedTheme,
              trimmed.isEmpty() ? QString() : QStringLiteral("pending"));
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



void SunReactorClient::setBusy(bool busy)
{
    if (m_isBusy != busy) {
        m_isBusy = busy;
        Q_EMIT busyChanged();
    }
}

void SunReactorClient::setErrorKind(ErrorKind kind)
{
    if (m_errorKind != kind) {
        m_errorKind = kind;
        Q_EMIT errorKindChanged();
    }
}

SunReactorClient::ErrorKind SunReactorClient::socketErrorKind(QLocalSocket::LocalSocketError socketError)
{
    switch (socketError) {
    case QLocalSocket::SocketAccessError:
        return ErrorKind::PermissionDenied;
    case QLocalSocket::SocketTimeoutError:
        return ErrorKind::Timeout;
    case QLocalSocket::DatagramTooLargeError:
        return ErrorKind::Protocol;
    case QLocalSocket::ServerNotFoundError:
    case QLocalSocket::ConnectionRefusedError:
    case QLocalSocket::PeerClosedError:
    default:
        return ErrorKind::NotRunning;
    }
}

void SunReactorClient::clearStaleStatus()
{
    m_graceTimer->stop();
    if (m_statusStale) {
        m_statusStale = false;
        Q_EMIT statusStaleChanged();
    }
    if (!m_dayCurve.isEmpty()) {
        m_dayCurve.clear();
        Q_EMIT dayCurveChanged();
    }
    clearStatus();
}

void SunReactorClient::onGraceTimerTriggered()
{
    if (!m_isConnected && m_statusStale) {
        clearStaleStatus();
    }
}

void SunReactorClient::setConnected(bool connected)
{
    setConnected(connected, true);
}

void SunReactorClient::setConnected(bool connected, bool allowGrace)
{
    if (!connected && (m_isConnected || m_disconnectedAtMs < 0)) {
        m_disconnectedAtMs = m_elapsed.elapsed();
    }
    if (connected) {
        m_graceTimer->stop();
        setErrorKind(ErrorKind::None);
        if (m_statusStale) {
            m_statusStale = false;
            Q_EMIT statusStaleChanged();
        }
        if (m_isConnected) {
            return;
        }
        m_isConnected = true;
        Q_EMIT connectionChanged(true);
        m_disconnectedAtMs = -1;
        // Whatever answers next may be a different (newer or older) daemon: its theme list is read again.
        m_themeListStale = true;
        return;
    }

    const bool wasConnected = m_isConnected;
    if (wasConnected) {
        m_isConnected = false;
        Q_EMIT connectionChanged(false);
        m_themeListRetryTimer->stop();
        // Whatever answers next may be a different (newer or older) daemon.
        setThemeRegistry(CapabilityUnknown);
        setDayCurveSupport(CapabilityUnknown);
        m_dayCurveDate.clear();
    }

    if (allowGrace && wasConnected && m_errorKind != ErrorKind::VersionMismatch) {
        // Keep the last monitors and curve visible (dimmed) across brief daemon
        // restarts, while resetting themeWarning/temperatureUnit immediately.
        bool scalarReset = false;
        if (!m_themeWarning.isEmpty()) {
            m_themeWarning.clear();
            scalarReset = true;
        }
        if (m_temperatureUnit != QLatin1String("celsius")) {
            m_temperatureUnit = QStringLiteral("celsius");
            scalarReset = true;
        }
        if (scalarReset) {
            Q_EMIT statusChanged();
        }
        if (!m_statusStale) {
            m_statusStale = true;
            Q_EMIT statusStaleChanged();
        }
        m_graceTimer->start(kStatusGraceMs);
        return;
    }

    if (!allowGrace || (!wasConnected && (!m_statusStale || m_errorKind == ErrorKind::VersionMismatch))) {
        clearStaleStatus();
    }
}

void SunReactorClient::fail(const QString &message, ErrorKind kind)
{
    setErrorKind(kind);
    m_lastError = message;
    Q_EMIT errorOccurred(m_lastError);
}
