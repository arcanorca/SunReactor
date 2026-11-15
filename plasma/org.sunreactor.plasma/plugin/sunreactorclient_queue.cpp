#include "sunreactorclient_private.h"

#include <algorithm>
#include <utility>

using namespace SunReactorClientPrivate;

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
void SunReactorClient::pruneStaleMutatingRequests()
{
    if (m_isConnected || m_requestQueue.isEmpty() || m_disconnectedAtMs < 0) {
        return;
    }
    const qint64 now = m_elapsed.elapsed();
    int dropped = 0;
    for (auto it = m_requestQueue.begin(); it != m_requestQueue.end();) {
        if (isMutatingRequest(it->object) && it->queuedAtMs > m_disconnectedAtMs
            && now - it->queuedAtMs > m_lastBackoffWindowMs) {
            it = m_requestQueue.erase(it);
            ++dropped;
        } else {
            ++it;
        }
    }
    if (dropped > 0) {
        fail(QStringLiteral("Dropped %1 queued command%2 while offline")
                 .arg(dropped)
                 .arg(dropped == 1 ? QString() : QStringLiteral("s")));
    }
}

void SunReactorClient::scheduleReconnect()
{
    const int delay = m_reconnectBackoffMs;
    m_lastBackoffWindowMs = delay;
    m_reconnectTimer->start(delay);
    m_reconnectBackoffMs = std::min(kMaxReconnectBackoffMs, delay * 2);
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

    // Per-monitor mutations are replaceable until they reach the daemon: the
    // latest value or hold action for one monitor is the only useful one.
    if (name == QStringLiteral("set_override") || name == QStringLiteral("set_monitor_limits")
        || name == QStringLiteral("clear_external_hold")) {
        const QString monitor = requestMonitorKey(requestObj);
        for (auto it = m_requestQueue.begin(); it != m_requestQueue.end(); ++it) {
            const QueuedRequest &queued = *it;
            if (queued.object.value(QStringLiteral("request")).toString() == name
                && requestMonitorKey(queued.object) == monitor) {
                // Removing the old edit before appending keeps commands for
                // the same monitor in the order the user issued them.
                m_requestQueue.erase(it);
                break;
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

    m_requestQueue.enqueue({requestObj, now, 0});

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
    m_requestQueue.prepend({requestObj, m_elapsed.elapsed(), 0});
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
    m_inFlightAttempts = queued.attempts;
    m_readBuffer.clear();

    m_socket->abort();
    m_watchdogTimer->start();
    m_socket->connectToServer(resolvedSocketPath());
}


