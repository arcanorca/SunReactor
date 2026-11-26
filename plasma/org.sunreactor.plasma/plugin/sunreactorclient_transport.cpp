#include "sunreactorclient_private.h"

#include <QJsonDocument>

#include "themenormalize.h"

using namespace SunReactorClientPrivate;

void SunReactorClient::onSocketConnected()
{
    sendInFlightRequest();
}

void SunReactorClient::sendInFlightRequest()
{
    if (m_socket->state() != QLocalSocket::ConnectedState) {
        transportError(QStringLiteral("Control socket disconnected before write"),
                       socketErrorKind(m_socket->error()));
        return;
    }

    const QByteArray payload = QJsonDocument(m_inFlightRequest).toJson(QJsonDocument::Compact);
    if (payload.size() > SunReactor::MAX_IPC_MESSAGE_BYTES) {
        transportError(QStringLiteral("Outgoing IPC request exceeded 64 KiB ceiling (%1 bytes)").arg(payload.size()),
                       ErrorKind::Protocol);
        return;
    }
    const QByteArray data = payload + '\n';

    const qint64 written = m_socket->write(data);
    if (written < 0 || written != data.size()) {
        const QString err = m_socket->errorString();
        transportError(err.isEmpty() ? QStringLiteral("Failed to write IPC request") : err,
                       socketErrorKind(m_socket->error()));
        return;
    }
    m_socket->flush();
}

void SunReactorClient::onSocketReadyRead()
{
    if (!m_isBusy || m_responseHandled) {
        return;
    }

    // Wire frame allows up to 64 KiB JSON payload plus its single trailing '\n'.
    constexpr qsizetype kMaxWireFrameBytes = SunReactor::MAX_IPC_MESSAGE_BYTES + 1;
    const qsizetype remaining = kMaxWireFrameBytes - m_readBuffer.size();
    if (remaining <= 0 || m_socket->bytesAvailable() > remaining) {
        transportError(QStringLiteral("Incoming IPC stream exceeded 64 KiB ceiling"), ErrorKind::Protocol);
        return;
    }
    m_readBuffer.append(m_socket->read(remaining));

    const int newlineIndex = m_readBuffer.indexOf('\n');
    if (newlineIndex < 0) {
        return;
    }
    if (newlineIndex > SunReactor::MAX_IPC_MESSAGE_BYTES) {
        transportError(QStringLiteral("Incoming IPC stream exceeded 64 KiB ceiling"), ErrorKind::Protocol);
        return;
    }

    const QByteArray line = m_readBuffer.left(newlineIndex).trimmed();
    m_readBuffer.clear();
    if (line.isEmpty()) {
        transportError(QStringLiteral("Empty response line from daemon"), ErrorKind::Protocol);
        return;
    }

    QJsonParseError parseError{};
    const QJsonDocument doc = QJsonDocument::fromJson(line, &parseError);
    if (parseError.error != QJsonParseError::NoError || !doc.isObject()) {
        transportError(QStringLiteral("Malformed JSON response from daemon: %1").arg(parseError.errorString()),
                       ErrorKind::Protocol);
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
            transportError(QStringLiteral("Malformed JSON response from daemon: %1").arg(parseError.errorString()),
                           ErrorKind::Protocol);
            return;
        }
    }

    transportError(QStringLiteral("Daemon closed the socket before replying"), ErrorKind::NotRunning);
}

void SunReactorClient::onSocketError(QLocalSocket::LocalSocketError socketError)
{
    if (socketError == QLocalSocket::PeerClosedError && m_responseHandled) {
        return;
    }

    transportError(m_socket->errorString(), socketErrorKind(socketError));
}

void SunReactorClient::onRequestTimeout()
{
    // The daemon is single threaded: after it acknowledges a command it applies
    // the change to the hardware before it accepts the next connection, which
    // can take longer than the base timeout. Giving up here closes a socket the
    // daemon is about to answer (Broken pipe on its side), replays the command
    // and, through setConnected(false), wipes the status the popup renders. A
    // client that was connected a moment ago and holds an open socket waits one
    // more grace period before it treats the daemon as gone.
    if (m_isConnected && !m_watchdogExtended && m_socket->state() == QLocalSocket::ConnectedState) {
        m_watchdogExtended = true;
        m_watchdogTimer->start(kBusyDaemonGraceMs);
        return;
    }
    const int waitedMs = m_watchdogExtended
        ? SunReactor::DEFAULT_REQUEST_TIMEOUT_MS + kBusyDaemonGraceMs
        : SunReactor::DEFAULT_REQUEST_TIMEOUT_MS;
    transportError(QStringLiteral("Request timed out after %1 ms").arg(waitedMs), ErrorKind::Timeout);
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
        fail(QStringLiteral("Unsupported IPC protocol version %1 (expected %2). Update the plasmoid or sunreactord.")
                 .arg(version)
                 .arg(SunReactor::PROTOCOL_VERSION),
             ErrorKind::VersionMismatch);
        return false;
    }

    const QString type = responseObj.value(QStringLiteral("response")).toString();

    const QString sent = m_inFlightRequest.value(QStringLiteral("request")).toString();
    if (type.isEmpty()) {
        fail(QStringLiteral("Missing response type from daemon"), ErrorKind::Protocol);
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
        if (responseObj.value(QStringLiteral("code")).toString() == QLatin1String("unsupported_version")) {
            fail(QStringLiteral("Unsupported IPC protocol version. Update the plasmoid or sunreactord."),
                 ErrorKind::VersionMismatch);
            return false;
        }
        // The transport answered with a valid protocol envelope even though
        // the requested operation failed; keep connectivity separate from the
        // daemon's capability/error state.
        setConnected(true);
        handleError(responseObj);
    } else {
        qWarning("SunReactorClient: ignoring unknown response type '%s'", qPrintable(type));
        setErrorKind(ErrorKind::None);
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

void SunReactorClient::transportError(const QString &message, ErrorKind kind)
{
    if (m_responseHandled) {
        return;
    }
    m_responseHandled = true;
    m_watchdogTimer->stop();
    m_readBuffer.clear();
    m_socket->abort();
    if (kind != ErrorKind::None) {
        setErrorKind(kind);
    }
    setConnected(false);
    if (!message.isEmpty()) {
        fail(message, m_errorKind == ErrorKind::None ? ErrorKind::NotRunning : m_errorKind);
    }
    if (!m_inFlightRequest.isEmpty()) {
        const qint64 now = m_elapsed.elapsed();
        const QString requestName = m_inFlightRequest.value(QStringLiteral("request")).toString();
        const bool mutating = isMutatingRequest(m_inFlightRequest);
        bool requeued = false;
        if (!mutating && m_errorKind != ErrorKind::VersionMismatch
            && m_inFlightAttempts < kMaxReadTransportRetries) {
            if (m_requestQueue.size() >= kMaxRequestQueue) {
                m_requestQueue.removeLast();
            }
            m_requestQueue.prepend({m_inFlightRequest, m_inFlightQueuedAtMs, m_inFlightAttempts + 1});
            requeued = true;
        } else if (mutating
                   && m_errorKind != ErrorKind::VersionMismatch
                   && now - m_inFlightQueuedAtMs <= kMaxOfflineMutatingAgeMs
                   && (m_inFlightQueuedAtMs <= m_disconnectedAtMs
                       || now - m_inFlightQueuedAtMs <= m_lastBackoffWindowMs)) {
            if (m_requestQueue.size() >= kMaxRequestQueue) {
                m_requestQueue.removeLast();
            }
            m_requestQueue.prepend({m_inFlightRequest, m_inFlightQueuedAtMs, m_inFlightAttempts});
            requeued = true;
        }
        if (!requeued && m_errorKind != ErrorKind::VersionMismatch) {
            // Keep the original transport error in lastError, but make a
            // capped read retry or a stale command visible to the UI too.
            const QString droppedMessage = QStringLiteral("Dropped %1 request after transport failure")
                .arg(requestName.isEmpty() ? QStringLiteral("unknown") : requestName);
            if (message.isEmpty()) {
                fail(droppedMessage, m_errorKind == ErrorKind::None ? ErrorKind::NotRunning : m_errorKind);
            } else {
                Q_EMIT errorOccurred(droppedMessage);
            }
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


