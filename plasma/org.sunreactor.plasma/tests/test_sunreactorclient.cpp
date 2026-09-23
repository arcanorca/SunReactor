/*
 * The tests talk to a QLocalServer standing in for the daemon: no real socket,
 * no hardware, no daemon process. They cover the framing rules of the control
 * protocol and, above all, that values the daemon does not report stay
 * unknown instead of being invented.
 */

#include <QCoreApplication>
#include <QList>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalServer>
#include <QLocalSocket>
#include <QSignalSpy>
#include <QTemporaryDir>
#include <QTest>
#include <QTimer>

#include <utility>

#include "sunreactorclient.h"
#include "sunreactortypes.h"

class TestSunReactorClient : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void initTestCase();
    void init();
    void cleanup();

    void testConnectAndParseStatus();
    void testUnknownBrightnessStaysUnknown();
    void testModePrecedence();
    void testStaleWeatherRemainsVisible();
    void testWeatherDayPhaseOverridesSolarElevation();
    void testForecastAndDetailsAreExposed();
    void testFramingAndChunkedStream();
    void test64KiBCeilingGuard();
    void testMalformedJsonRecovery();
    void testCommandSequencing();
    void testOverrideRequestsAreWellFormed();
    void testAcknowledgedStatusDoesNotLoop();
    void testRequestTimeout();

private:
    /// Answers every request with one status payload.
    void serveStatus(const QJsonObject &statusObj);
    /// Acknowledges every request and records what was asked for.
    void serveAck();
    static QJsonObject statusResponse(const QJsonObject &statusObj);

    QList<QJsonObject> m_received;
    QTemporaryDir m_tempDir;
    QString m_socketPath;
    QLocalServer *m_server = nullptr;
};

void TestSunReactorClient::initTestCase()
{
    QVERIFY(m_tempDir.isValid());
    m_socketPath = m_tempDir.filePath(QStringLiteral("test_sunreactor.sock"));
}

void TestSunReactorClient::init()
{
    m_server = new QLocalServer(this);
    QLocalServer::removeServer(m_socketPath);
    QVERIFY2(m_server->listen(m_socketPath), qPrintable(m_server->errorString()));
}

void TestSunReactorClient::cleanup()
{
    m_received.clear();
    if (m_server) {
        m_server->close();
        delete m_server;
        m_server = nullptr;
    }
    QLocalServer::removeServer(m_socketPath);
}

QJsonObject TestSunReactorClient::statusResponse(const QJsonObject &statusObj)
{
    QJsonObject response;
    response[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
    response[QStringLiteral("response")] = QStringLiteral("status");
    response[QStringLiteral("status")] = statusObj;
    return response;
}

void TestSunReactorClient::serveStatus(const QJsonObject &statusObj)
{
    connect(m_server, &QLocalServer::newConnection, this, [this, statusObj]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        QVERIFY(socket != nullptr);

        connect(socket, &QLocalSocket::readyRead, socket, [socket, statusObj]() {
            if (!socket->readAll().contains('\n')) {
                return;
            }
            socket->write(QJsonDocument(statusResponse(statusObj)).toJson(QJsonDocument::Compact) + '\n');
            socket->flush();
            socket->disconnectFromServer();
        });
    });
}

void TestSunReactorClient::serveAck()
{
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        QVERIFY(socket != nullptr);

        connect(socket, &QLocalSocket::readyRead, socket, [this, socket]() {
            const QByteArray data = socket->readAll();
            if (!data.contains('\n')) {
                return;
            }
            const QJsonObject request = QJsonDocument::fromJson(data).object();
            m_received.append(request);

            QJsonObject response;
            if (request.value(QStringLiteral("request")).toString() == QStringLiteral("status")) {
                response = statusResponse(QJsonObject{});
            } else {
                response[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
                response[QStringLiteral("response")] = QStringLiteral("ack");
                response[QStringLiteral("message")] = QStringLiteral("ok");
            }
            socket->write(QJsonDocument(response).toJson(QJsonDocument::Compact) + '\n');
            socket->flush();
            socket->disconnectFromServer();
        });
    });
}

void TestSunReactorClient::testOverrideRequestsAreWellFormed()
{
    serveAck();

    SunReactorClient client(m_socketPath);
    client.setGlobalOverride(150, 0);
    client.setMonitorOverride(QStringLiteral("  desk  "), -5, 30);
    client.clearAllOverrides();

    // Each accepted command is followed by a status read, so count only the
    // commands themselves.
    const auto commandsSoFar = [this]() {
        QList<QJsonObject> commands;
        for (const QJsonObject &request : std::as_const(m_received)) {
            if (request.value(QStringLiteral("request")).toString() != QStringLiteral("status")) {
                commands.append(request);
            }
        }
        return commands;
    };

    QTRY_VERIFY_WITH_TIMEOUT(commandsSoFar().size() >= 3, 5000);

    const QList<QJsonObject> commands = commandsSoFar();
    QCOMPARE(commands.size(), 3);

    const QJsonObject global = commands.at(0);
    QCOMPARE(global.value(QStringLiteral("request")).toString(), QStringLiteral("set_override"));
    QVERIFY(global.value(QStringLiteral("monitor_id")).isNull());
    // Out-of-range percentages are clamped to the 0..100 scale.
    QCOMPARE(global.value(QStringLiteral("percent")).toInt(), 100);
    // Zero minutes means "no expiry", which the protocol spells as null.
    QVERIFY(global.value(QStringLiteral("minutes")).isNull());

    const QJsonObject perMonitor = commands.at(1);
    QCOMPARE(perMonitor.value(QStringLiteral("monitor_id")).toString(), QStringLiteral("desk"));
    QCOMPARE(perMonitor.value(QStringLiteral("percent")).toInt(), 0);
    QCOMPARE(perMonitor.value(QStringLiteral("minutes")).toInt(), 30);

    const QJsonObject cleared = commands.at(2);
    QCOMPARE(cleared.value(QStringLiteral("request")).toString(), QStringLiteral("clear_override"));
    QVERIFY(cleared.value(QStringLiteral("monitor_id")).isNull());
    // false means every override, global and per display; true would keep the
    // per-display ones the sliders create.
    QCOMPARE(cleared.value(QStringLiteral("global")).toBool(), false);
}

void TestSunReactorClient::testAcknowledgedStatusDoesNotLoop()
{
    // A peer that answers every request - including a status read - with an
    // acknowledgement must not make the client chase its own tail.
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [this, socket]() {
            const QByteArray data = socket->readAll();
            if (!data.contains('\n')) {
                return;
            }
            m_received.append(QJsonDocument::fromJson(data).object());

            QJsonObject ack;
            ack[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
            ack[QStringLiteral("response")] = QStringLiteral("ack");
            ack[QStringLiteral("message")] = QStringLiteral("ok");
            socket->write(QJsonDocument(ack).toJson(QJsonDocument::Compact) + '\n');
            socket->flush();
            socket->disconnectFromServer();
        });
    });

    SunReactorClient client(m_socketPath);
    client.suspend(30);

    QTRY_VERIFY_WITH_TIMEOUT(m_received.size() >= 2, 2000);
    const int settled = m_received.size();

    // Give the event loop a second to run wild if the guard is missing.
    QTest::qWait(1000);
    QCOMPARE(m_received.size(), settled);
}

void TestSunReactorClient::testConnectAndParseStatus()
{
    QJsonObject weather;
    weather[QStringLiteral("enabled")] = true;
    weather[QStringLiteral("state")] = QStringLiteral("ready");
    weather[QStringLiteral("active")] = true;
    weather[QStringLiteral("stale")] = false;
    weather[QStringLiteral("cloud_cover_percent")] = 40;
    weather[QStringLiteral("multiplier")] = 0.92;
    weather[QStringLiteral("temperature")] = 21.4;
    weather[QStringLiteral("condition")] = QStringLiteral("partly_cloudy");
    weather[QStringLiteral("day_phase")] = QStringLiteral("day");

    QJsonObject external;
    external[QStringLiteral("logical_id")] = QStringLiteral("desk");
    external[QStringLiteral("backend")] = QStringLiteral("ddc");
    external[QStringLiteral("enabled")] = true;
    external[QStringLiteral("last_applied_percent")] = 70;
    external[QStringLiteral("override_percent")] = 75;
    external[QStringLiteral("topology")] = QStringLiteral("present");

    QJsonObject internal;
    internal[QStringLiteral("logical_id")] = QStringLiteral("laptop");
    internal[QStringLiteral("backend")] = QStringLiteral("backlight");
    internal[QStringLiteral("enabled")] = true;
    internal[QStringLiteral("last_applied_percent")] = 50;

    QJsonObject status;
    status[QStringLiteral("suspended")] = false;
    status[QStringLiteral("manual_override_active")] = true;
    status[QStringLiteral("solar_elevation")] = 28.45;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("sunrise_epoch_s")] = 1773977880;
    status[QStringLiteral("sunset_epoch_s")] = 1774026120;
    status[QStringLiteral("global_override_until_epoch_s")] = 1774002000;
    status[QStringLiteral("weather")] = weather;
    status[QStringLiteral("monitors")] = QJsonArray{external, internal};

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QVERIFY(client.isConnected());
    QCOMPARE(client.mode(), SunReactorClient::Manual);
    QVERIFY(client.isOverrideActive());
    QCOMPARE(client.overrideUntilEpochS(), 1774002000);
    QVERIFY(client.isDaylight());

    // Solar noon is the midpoint of the two reported events.
    QCOMPARE(client.solarNoonEpochS(), 1774002000);

    QVERIFY(client.weatherEnabled());
    QVERIFY(client.hasWeatherReading());
    QVERIFY(!client.weatherIsNight());
    QCOMPARE(client.weatherCondition(), QStringLiteral("partly_cloudy"));
    QCOMPARE(client.cloudCoverPercent(), 40);
    QVERIFY(client.hasTemperature());
    QCOMPARE(qRound(client.temperatureC()), 21);

    QCOMPARE(client.monitors().size(), 2);
    const QVariantMap first = client.monitors().at(0).toMap();
    QCOMPARE(first.value(QStringLiteral("logicalId")).toString(), QStringLiteral("desk"));
    QCOMPARE(first.value(QStringLiteral("backend")).toString(), QStringLiteral("ddc"));
    QVERIFY(first.value(QStringLiteral("hasOverride")).toBool());
    // An override is what the user asked for, so it is what the slider shows.
    QCOMPARE(first.value(QStringLiteral("percent")).toInt(), 75);
    QCOMPARE(first.value(QStringLiteral("appliedPercent")).toInt(), 70);
    QVERIFY(!first.value(QStringLiteral("unreachable")).toBool());

    const QVariantMap second = client.monitors().at(1).toMap();
    QVERIFY(!second.value(QStringLiteral("hasOverride")).toBool());
    QCOMPARE(second.value(QStringLiteral("percent")).toInt(), 50);

    // No global override percentage: the mean of what was applied.
    QCOMPARE(client.globalPercent(), 60);
}

void TestSunReactorClient::testUnknownBrightnessStaysUnknown()
{
    QJsonObject monitor;
    monitor[QStringLiteral("logical_id")] = QStringLiteral("desk");
    monitor[QStringLiteral("backend")] = QStringLiteral("ddc");
    monitor[QStringLiteral("enabled")] = true;
    monitor[QStringLiteral("last_applied_percent")] = QJsonValue(QJsonValue::Null);
    monitor[QStringLiteral("override_percent")] = QJsonValue(QJsonValue::Null);
    monitor[QStringLiteral("backoff_until_epoch_s")] = 1774000000;

    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("monitors")] = QJsonArray{monitor};

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);
    QVERIFY(statusSpy.wait(2000));

    const QVariantMap first = client.monitors().at(0).toMap();
    QCOMPARE(first.value(QStringLiteral("percent")).toInt(), SunReactor::UNKNOWN_PERCENT);
    QCOMPARE(first.value(QStringLiteral("appliedPercent")).toInt(), SunReactor::UNKNOWN_PERCENT);
    // The backoff deadline is in the future, so the display is not answering.
    QVERIFY(first.value(QStringLiteral("unreachable")).toBool());
    QCOMPARE(first.value(QStringLiteral("backoffRemainingS")).toLongLong(), 1600);

    QCOMPARE(client.globalPercent(), SunReactor::UNKNOWN_PERCENT);
}

void TestSunReactorClient::testModePrecedence()
{
    QJsonObject status;
    status[QStringLiteral("suspended")] = true;
    status[QStringLiteral("manual_override_active")] = true;
    status[QStringLiteral("desktop_idle_dimmed")] = true;
    status[QStringLiteral("suspend_until_epoch_s")] = 1774002000;

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);
    QVERIFY(statusSpy.wait(2000));

    // Paused outranks every other state: nothing is being adjusted at all.
    QCOMPARE(client.mode(), SunReactorClient::Paused);
    QCOMPARE(client.suspendUntilEpochS(), 1774002000);
}

void TestSunReactorClient::testStaleWeatherRemainsVisible()
{
    QJsonObject weather;
    weather[QStringLiteral("enabled")] = true;
    weather[QStringLiteral("state")] = QStringLiteral("stale");
    weather[QStringLiteral("active")] = false;
    weather[QStringLiteral("stale")] = true;
    weather[QStringLiteral("cloud_cover_percent")] = 92;
    weather[QStringLiteral("multiplier")] = QJsonValue(QJsonValue::Null);
    weather[QStringLiteral("temperature")] = 12.8;
    weather[QStringLiteral("condition")] = QStringLiteral("rain");

    QJsonObject status;
    status[QStringLiteral("weather")] = weather;

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QVERIFY(client.weatherEnabled());
    QVERIFY(client.hasWeatherReading());
    QVERIFY(client.weatherStale());
    QCOMPARE(client.weatherState(), QStringLiteral("stale"));
    QCOMPARE(client.cloudCoverPercent(), 92);
    // A missing multiplier means "no effect", never a guess.
    QCOMPARE(client.weatherMultiplier(), 1.0);
    QCOMPARE(qRound(client.temperatureC()), 13);
    QCOMPARE(client.weatherCondition(), QStringLiteral("rain"));
}

void TestSunReactorClient::testWeatherDayPhaseOverridesSolarElevation()
{
    QJsonObject weather;
    weather[QStringLiteral("enabled")] = true;
    weather[QStringLiteral("condition")] = QStringLiteral("clear");
    weather[QStringLiteral("day_phase")] = QStringLiteral("night");

    QJsonObject status;
    // The sun is barely up, but the provider says the observation is a night one.
    status[QStringLiteral("solar_elevation")] = 0.4;
    status[QStringLiteral("weather")] = weather;

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QVERIFY(client.isDaylight());
    QVERIFY(client.weatherIsNight());
}

void TestSunReactorClient::testForecastAndDetailsAreExposed()
{
    QJsonObject usable;
    usable[QStringLiteral("dt_epoch_s")] = 1774002000;
    usable[QStringLiteral("temperature")] = 18.6;
    usable[QStringLiteral("condition")] = QStringLiteral("rain");
    usable[QStringLiteral("day_phase")] = QStringLiteral("night");
    usable[QStringLiteral("cloud_cover_percent")] = 88;
    usable[QStringLiteral("precipitation_percent")] = 60;

    // No temperature: nothing to draw, so it must not reach the strip.
    QJsonObject unusable;
    unusable[QStringLiteral("dt_epoch_s")] = 1774012800;
    unusable[QStringLiteral("temperature")] = QJsonValue(QJsonValue::Null);

    QJsonObject airQuality;
    airQuality[QStringLiteral("us_aqi")] = 42;
    airQuality[QStringLiteral("pm2_5")] = 9.7;

    QJsonObject details;
    details[QStringLiteral("feels_like")] = 16.2;
    details[QStringLiteral("humidity_percent")] = 71;
    details[QStringLiteral("wind_speed_mps")] = 3.4;
    details[QStringLiteral("pressure_hpa")] = QJsonValue(QJsonValue::Null);
    details[QStringLiteral("air_quality")] = airQuality;

    QJsonObject weather;
    weather[QStringLiteral("enabled")] = true;
    weather[QStringLiteral("condition")] = QStringLiteral("rain");
    weather[QStringLiteral("forecast")] = QJsonArray{usable, unusable};
    weather[QStringLiteral("details")] = details;

    QJsonObject status;
    status[QStringLiteral("weather")] = weather;

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);
    QVERIFY(statusSpy.wait(2000));

    QCOMPARE(client.forecast().size(), 1);
    const QVariantMap point = client.forecast().at(0).toMap();
    QCOMPARE(point.value(QStringLiteral("epochS")).toLongLong(), 1774002000);
    QCOMPARE(qRound(point.value(QStringLiteral("temperatureC")).toDouble()), 19);
    QCOMPARE(point.value(QStringLiteral("condition")).toString(), QStringLiteral("rain"));
    QVERIFY(point.value(QStringLiteral("isNight")).toBool());
    QCOMPARE(point.value(QStringLiteral("precipitationPercent")).toInt(), 60);

    const QVariantMap secondary = client.weatherDetails();
    QCOMPARE(qRound(secondary.value(QStringLiteral("feelsLikeC")).toDouble()), 16);
    QCOMPARE(secondary.value(QStringLiteral("humidityPercent")).toInt(), 71);
    QCOMPARE(secondary.value(QStringLiteral("airQualityIndex")).toInt(), 42);
    // A reading the provider did not send stays absent rather than becoming 0.
    QVERIFY(!secondary.contains(QStringLiteral("pressureHpa")));
}

void TestSunReactorClient::testFramingAndChunkedStream()
{
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [socket]() {
            if (!socket->readAll().contains('\n')) {
                return;
            }

            QJsonObject status;
            status[QStringLiteral("solar_elevation")] = 14.2;
            const QByteArray full = QJsonDocument(statusResponse(status)).toJson(QJsonDocument::Compact) + '\n';

            const int split1 = full.size() / 3;
            const int split2 = (full.size() * 2) / 3;

            socket->write(full.left(split1));
            socket->flush();

            QTimer::singleShot(50, socket, [socket, full, split1, split2]() {
                socket->write(full.mid(split1, split2 - split1));
                socket->flush();

                QTimer::singleShot(50, socket, [socket, full, split2]() {
                    socket->write(full.mid(split2));
                    socket->flush();
                    socket->disconnectFromServer();
                });
            });
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QVERIFY(client.isConnected());
    QVERIFY(client.isDaylight());
}

void TestSunReactorClient::test64KiBCeilingGuard()
{
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [socket]() {
            socket->readAll();
            // 65 KiB without a single newline: a stream that will never frame.
            socket->write(QByteArray(65 * 1024 + 1, 'X'));
            socket->flush();
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy errorSpy(&client, &SunReactorClient::errorOccurred);

    QVERIFY(errorSpy.wait(2000));
    QVERIFY(!client.isConnected());
    QVERIFY(client.lastError().contains(QStringLiteral("exceeded 64 KiB ceiling")));
}

void TestSunReactorClient::testMalformedJsonRecovery()
{
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [socket]() {
            socket->readAll();
            socket->write("{\"bad_json: missing_brace\n");
            socket->flush();
            socket->disconnectFromServer();
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy errorSpy(&client, &SunReactorClient::errorOccurred);

    QVERIFY(errorSpy.wait(2000));
    QVERIFY(client.lastError().contains(QStringLiteral("Malformed JSON")));
}

void TestSunReactorClient::testCommandSequencing()
{
    int requestCounter = 0;

    connect(m_server, &QLocalServer::newConnection, this, [this, &requestCounter]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [socket, &requestCounter]() {
            const QByteArray data = socket->readAll();
            if (!data.contains('\n')) {
                return;
            }

            ++requestCounter;
            const QString name = QJsonDocument::fromJson(data)
                                     .object()
                                     .value(QStringLiteral("request"))
                                     .toString();

            if (name == QStringLiteral("suspend")) {
                QJsonObject ack;
                ack[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
                ack[QStringLiteral("response")] = QStringLiteral("ack");
                ack[QStringLiteral("message")] = QStringLiteral("suspended");
                socket->write(QJsonDocument(ack).toJson(QJsonDocument::Compact) + '\n');
            } else {
                QJsonObject status;
                status[QStringLiteral("suspended")] = true;
                socket->write(QJsonDocument(statusResponse(status)).toJson(QJsonDocument::Compact) + '\n');
            }

            socket->flush();
            socket->disconnectFromServer();
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));

    // An accepted command must be followed by a fresh read of the real state.
    client.suspend(30);

    QVERIFY(statusSpy.wait(3000));
    QCOMPARE(client.mode(), SunReactorClient::Paused);
    QVERIFY(requestCounter >= 2);
}

void TestSunReactorClient::testRequestTimeout()
{
    connect(m_server, &QLocalServer::newConnection, this, [this]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        connect(socket, &QLocalSocket::readyRead, socket, [socket]() {
            socket->readAll();
            // Accepts the request and then says nothing at all.
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy errorSpy(&client, &SunReactorClient::errorOccurred);

    QVERIFY(errorSpy.wait(4000));
    QVERIFY(!client.isConnected());
    QCOMPARE(client.mode(), SunReactorClient::Offline);
    QVERIFY(client.lastError().contains(QStringLiteral("timed out")));
}

QTEST_MAIN(TestSunReactorClient)
#include "test_sunreactorclient.moc"
