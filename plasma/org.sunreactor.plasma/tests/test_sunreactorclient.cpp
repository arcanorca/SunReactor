/*
 * The tests talk to a QLocalServer standing in for the daemon: no real socket,
 * no hardware, no daemon process. They cover the framing rules of the control
 * protocol and, above all, that values the daemon does not report stay
 * unknown instead of being invented.
 */

#include <QCoreApplication>
#include <QFileInfo>
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

#include <memory>

#include <functional>
#include <utility>

#include "sunreactorclient.h"
#include "sunreactortypes.h"
#include "themenormalize.h"

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
    void testMonitorLimitsRequestsAreWellFormed();
    void testAcknowledgedStatusDoesNotLoop();
    void testRequestTimeout();
    void testThemeHotSwapping();

    void testNewThemeIsNormalized();
    void testMalformedThemeFallsBackToDefaults();
    void testLegacyThemeUsesExactTableOnly();
    void testOldDaemonDegradesWithoutSideEffects();
    void testDayCurveParsedAndNotPolled();
    void testDayCurveFollowsLimitChanges();
    void testThemeListAndFetch();
    void testPinnedThemeIsKeptFetched();
    void testThemeWarningIsOptional();
    void testThemesFolderFollowsXdgConfigHome();
    void testHarnessConfigNeedsHarness();

private:
    using Handler = std::function<QJsonObject(const QJsonObject &request)>;
    /// Answers every request through `handler` and records what was asked for.
    void serveWith(Handler handler);
    [[nodiscard]] int countRequests(const QString &name) const;
    static QJsonObject envelope(const QString &kind, const QJsonObject &fields = {});
    static QJsonObject errorResponse(const QString &code, const QString &message);
    static QJsonObject fullTheme(const QString &id);
    static QJsonObject dayCurveBody();

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

void TestSunReactorClient::testMonitorLimitsRequestsAreWellFormed()
{
    serveAck();

    SunReactorClient client(m_socketPath);
    client.setMonitorLimits(QStringLiteral("  desk  "), -5, 120);

    // Reads (status, and the day curve a limit change invalidates) are not commands.
    const auto commandsSoFar = [this]() {
        QList<QJsonObject> commands;
        for (const QJsonObject &request : std::as_const(m_received)) {
            const QString name = request.value(QStringLiteral("request")).toString();
            if (name != QStringLiteral("status") && name != QStringLiteral("day_curve")) {
                commands.append(request);
            }
        }
        return commands;
    };

    QTRY_VERIFY_WITH_TIMEOUT(!commandsSoFar().isEmpty(), 5000);

    const QList<QJsonObject> commands = commandsSoFar();
    QCOMPARE(commands.size(), 1);

    const QJsonObject limits = commands.at(0);
    QCOMPARE(limits.value(QStringLiteral("request")).toString(), QStringLiteral("set_monitor_limits"));
    QCOMPARE(limits.value(QStringLiteral("monitor_id")).toString(), QStringLiteral("desk"));
    QCOMPARE(limits.value(QStringLiteral("min_pct")).toInt(), 0);
    QCOMPARE(limits.value(QStringLiteral("max_pct")).toInt(), 100);
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

    QJsonObject theme;
    theme[QStringLiteral("name")] = QStringLiteral("Amber");
    theme[QStringLiteral("accent")] = QStringLiteral("#ffb000");
    theme[QStringLiteral("secondary_accent")] = QStringLiteral("#dc8c00");
    theme[QStringLiteral("bg")] = QStringLiteral("#0f0a05");
    theme[QStringLiteral("fg")] = QStringLiteral("#ffd296");
    theme[QStringLiteral("text_muted")] = QStringLiteral("#b47832");
    status[QStringLiteral("theme")] = theme;

    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QVERIFY(client.isConnected());
    QCOMPARE(client.themeName(), QStringLiteral("Amber"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#ffb000"));
    QCOMPARE(client.themeSecondaryAccent(), QStringLiteral("#dc8c00"));
    QCOMPARE(client.themeBg(), QStringLiteral("#0f0a05"));
    QCOMPARE(client.themeFg(), QStringLiteral("#ffd296"));
    QCOMPARE(client.themeTextMuted(), QStringLiteral("#b47832"));
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

void TestSunReactorClient::testThemeHotSwapping()
{
    QJsonObject currentTheme;
    currentTheme[QStringLiteral("name")] = QStringLiteral("nothing");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#FFFFFF");
    currentTheme[QStringLiteral("secondary_accent")] = QStringLiteral("#E50914");
    currentTheme[QStringLiteral("bg")] = QStringLiteral("#121216");
    currentTheme[QStringLiteral("fg")] = QStringLiteral("#FFFFFF");
    currentTheme[QStringLiteral("text_muted")] = QStringLiteral("#888888");

    connect(m_server, &QLocalServer::newConnection, this, [this, &currentTheme]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        if (!socket) return;
        connect(socket, &QLocalSocket::readyRead, socket, [socket, &currentTheme]() {
            if (!socket->readAll().contains('\n')) {
                return;
            }
            QJsonObject status;
            status[QStringLiteral("theme")] = currentTheme;
            socket->write(QJsonDocument(statusResponse(status)).toJson(QJsonDocument::Compact) + '\n');
            socket->flush();
            socket->disconnectFromServer();
        });
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);

    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("nothing"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#FFFFFF"));
    QCOMPARE(client.themeSecondaryAccent(), QStringLiteral("#E50914"));

    // Hot-swap to Handheld
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("handheld");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#8B1D42");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("handheld"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#8B1D42"));

    // Hot-swap to Amiga
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("amiga");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#FF8800");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("amiga"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#FF8800"));

    // Hot-swap to Braun
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("braun");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#FF5500");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("braun"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#FF5500"));

    // Hot-swap to VFD
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("vfd_hifi");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#00F0A8");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("vfd_hifi"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#00F0A8"));

    // Hot-swap to Nixie
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("nixie");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#FF7A18");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("nixie"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#FF7A18"));

    // Hot-swap to ThinkPad
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("thinkpad");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#DA291C");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("thinkpad"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#DA291C"));

    // Hot-swap to Unix Workstation
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("unix_workstation");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#4580A0");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("unix_workstation"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#4580A0"));

    // Hot-swap to Commodore 64
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("commodore64");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("#8870E6");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("commodore64"));
    QCOMPARE(client.themeAccent(), QStringLiteral("#8870E6"));

    // Hot-swap to Modern
    statusSpy.clear();
    currentTheme[QStringLiteral("name")] = QStringLiteral("modern");
    currentTheme[QStringLiteral("accent")] = QStringLiteral("");
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeName(), QStringLiteral("modern"));
    QCOMPARE(client.themeAccent(), QStringLiteral(""));
}


// --- theme registry, day curve, old daemons --------------------------------

void TestSunReactorClient::serveWith(Handler handler)
{
    connect(m_server, &QLocalServer::newConnection, this, [this, handler]() {
        QLocalSocket *socket = m_server->nextPendingConnection();
        if (!socket) {
            return;
        }
        auto buffer = std::make_shared<QByteArray>();
        connect(socket, &QLocalSocket::readyRead, socket, [this, socket, handler, buffer]() {
            buffer->append(socket->readAll());
            if (!buffer->contains('\n')) {
                return;
            }
            const QJsonObject request = QJsonDocument::fromJson(buffer->left(buffer->indexOf('\n'))).object();
            m_received.append(request);
            socket->write(QJsonDocument(handler(request)).toJson(QJsonDocument::Compact) + '\n');
            socket->flush();
            socket->disconnectFromServer();
        });
    });
}

int TestSunReactorClient::countRequests(const QString &name) const
{
    int n = 0;
    for (const QJsonObject &request : m_received) {
        if (request.value(QStringLiteral("request")).toString() == name) {
            ++n;
        }
    }
    return n;
}

QJsonObject TestSunReactorClient::envelope(const QString &kind, const QJsonObject &fields)
{
    QJsonObject response = fields;
    response[QStringLiteral("version")] = static_cast<int>(SunReactor::PROTOCOL_VERSION);
    response[QStringLiteral("response")] = kind;
    return response;
}

QJsonObject TestSunReactorClient::errorResponse(const QString &code, const QString &message)
{
    return envelope(QStringLiteral("error"),
                    {{QStringLiteral("code"), code}, {QStringLiteral("message"), message}});
}

QJsonObject TestSunReactorClient::fullTheme(const QString &id)
{
    QJsonObject palette{
        {QStringLiteral("bg"), QStringLiteral("#2E3440")},
        {QStringLiteral("fg"), QStringLiteral("#d8dee9")},
        {QStringLiteral("accent"), QStringLiteral("#88c0d0")},
        {QStringLiteral("secondary_accent"), QStringLiteral("#81a1c1")},
        {QStringLiteral("border_active"), QStringLiteral("#88c0d0")},
        {QStringLiteral("border_inactive"), QStringLiteral("#647387")},
        {QStringLiteral("text_muted"), QStringLiteral("#8fbcbb")},
        {QStringLiteral("success"), QStringLiteral("#a3be8c")},
        {QStringLiteral("warning"), QStringLiteral("#ebcb8b")},
        {QStringLiteral("error"), QStringLiteral("#bf616a")},
    };
    QJsonObject widget{
        {QStringLiteral("renderer"), QStringLiteral("terminal")},
        {QStringLiteral("density"), QStringLiteral("comfortable")},
        {QStringLiteral("modules"), QJsonArray{QStringLiteral("displays"), QStringLiteral("curve"),
                                               QStringLiteral("bogus"), QStringLiteral("displays")}},
    };
    return {
        {QStringLiteral("name"), id == QStringLiteral("nord") ? QStringLiteral("Nord") : id},
        {QStringLiteral("accent"), QStringLiteral("#88c0d0")},
        {QStringLiteral("secondary_accent"), QStringLiteral("#81a1c1")},
        {QStringLiteral("bg"), QStringLiteral("#2e3440")},
        {QStringLiteral("fg"), QStringLiteral("#d8dee9")},
        {QStringLiteral("text_muted"), QStringLiteral("#8fbcbb")},
        {QStringLiteral("id"), id},
        {QStringLiteral("family"), QStringLiteral("terminal")},
        {QStringLiteral("glyph_style"), QStringLiteral("rounded")},
        {QStringLiteral("palette"), palette},
        {QStringLiteral("widget"), widget},
    };
}

QJsonObject TestSunReactorClient::dayCurveBody()
{
    QJsonArray samples{
        QJsonObject{{QStringLiteral("t"), 1773961200}, {QStringLiteral("pct"), 3}, {QStringLiteral("pct_no_weather"), 3}},
        QJsonObject{{QStringLiteral("t"), 1773961800}, {QStringLiteral("pct"), 140}}, // clamped, no-weather = pct
        QJsonObject{{QStringLiteral("t"), 1773961800}, {QStringLiteral("pct"), 9}},   // not moving forward: dropped
        QJsonObject{{QStringLiteral("t"), 1773962400}, {QStringLiteral("pct"), QJsonValue::Null}}, // no value: dropped
        QJsonObject{{QStringLiteral("t"), 1773963000}, {QStringLiteral("pct"), 30}, {QStringLiteral("pct_no_weather"), 34}},
    };
    QJsonObject monitor{
        {QStringLiteral("monitor_id"), QStringLiteral("mi-monitor")},
        {QStringLiteral("min_pct"), 1},
        {QStringLiteral("max_pct"), 50},
        {QStringLiteral("samples"), samples},
        {QStringLiteral("milestones"), QJsonArray{QJsonObject{{QStringLiteral("id"), QStringLiteral("rise_50")},
                                                               {QStringLiteral("label"), QStringLiteral("Rise 50%")},
                                                               {QStringLiteral("t"), 1773963000},
                                                               {QStringLiteral("pct"), 34}}}},
    };
    QJsonObject sun{
        {QStringLiteral("dawn"), 1773958000},
        {QStringLiteral("sunrise"), 1773960000},
        {QStringLiteral("solar_noon"), 1773980000},
        {QStringLiteral("sunset"), 1774000000},
        {QStringLiteral("dusk"), QJsonValue::Null}, // polar edge: structured, not invented
    };
    return {
        {QStringLiteral("date"), QStringLiteral("2026-03-20")},
        {QStringLiteral("step_minutes"), 10},
        {QStringLiteral("generated_at_epoch_s"), 1773998400},
        {QStringLiteral("weather_multiplier"), 0.88},
        {QStringLiteral("sun"), sun},
        {QStringLiteral("monitors"), QJsonArray{monitor, QJsonObject{{QStringLiteral("min_pct"), 3}}}},
    };
}

void TestSunReactorClient::testNewThemeIsNormalized()
{
    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("theme")] = fullTheme(QStringLiteral("nord"));
    serveStatus(status);

    SunReactorClient client(m_socketPath);
    QSignalSpy themeSpy(&client, &SunReactorClient::themeChanged);
    QVERIFY(themeSpy.wait(2000));

    const QVariantMap theme = client.theme();
    QCOMPARE(theme.value(QStringLiteral("id")).toString(), QStringLiteral("nord"));
    QCOMPARE(theme.value(QStringLiteral("family")).toString(), QStringLiteral("terminal"));
    QCOMPARE(theme.value(QStringLiteral("legacy")).toBool(), false);
    const QVariantMap palette = theme.value(QStringLiteral("palette")).toMap();
    // Colours are lower-cased; keys are camelCase for QML.
    QCOMPARE(palette.value(QStringLiteral("bg")).toString(), QStringLiteral("#2e3440"));
    QCOMPARE(palette.value(QStringLiteral("secondaryAccent")).toString(), QStringLiteral("#81a1c1"));
    QCOMPARE(palette.value(QStringLiteral("borderInactive")).toString(), QStringLiteral("#647387"));
    QCOMPARE(palette.value(QStringLiteral("error")).toString(), QStringLiteral("#bf616a"));

    const QVariantMap widget = theme.value(QStringLiteral("widget")).toMap();
    QCOMPARE(widget.value(QStringLiteral("renderer")).toString(), QStringLiteral("terminal"));
    QCOMPARE(widget.value(QStringLiteral("density")).toString(), QStringLiteral("comfortable"));
    // Omitted keys come from the terminal family defaults.
    QCOMPARE(widget.value(QStringLiteral("font")).toString(), QStringLiteral("mono"));
    QCOMPARE(widget.value(QStringLiteral("radius")).toInt(), 0);
    QCOMPARE(widget.value(QStringLiteral("dither")).toString(), QStringLiteral("none"));
    QVERIFY(widget.value(QStringLiteral("shades")).toList().isEmpty());
    // Unknown and duplicate module ids are dropped, order is kept.
    QCOMPARE(widget.value(QStringLiteral("modules")).toStringList(),
             (QStringList{QStringLiteral("displays"), QStringLiteral("curve")}));
    QCOMPARE(client.themeRegistry(), SunReactorClient::CapabilitySupported);

    // The same status again must not re-announce the theme.
    themeSpy.clear();
    client.queryStatus();
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(themeSpy.count(), 0);
}

void TestSunReactorClient::testMalformedThemeFallsBackToDefaults()
{
    using namespace SunReactor::ThemeData;
    QJsonObject theme = fullTheme(QStringLiteral("odd"));
    theme[QStringLiteral("family")] = QStringLiteral("pixel");
    theme[QStringLiteral("widget")] = QJsonObject{
        {QStringLiteral("renderer"), QStringLiteral("hologram")},
        {QStringLiteral("radius"), 99},
        {QStringLiteral("pixel_scale"), 9},
        {QStringLiteral("glow"), 3.0},
        {QStringLiteral("shades"), QJsonArray{QStringLiteral("#000000"), QStringLiteral("red")}},
        {QStringLiteral("modules"), QJsonArray{QStringLiteral("nope")}},
    };
    const QVariantMap out = normalizeTheme(theme);
    const QVariantMap widget = out.value(QStringLiteral("widget")).toMap();
    QCOMPARE(widget.value(QStringLiteral("renderer")).toString(), QStringLiteral("pixel"));
    QCOMPARE(widget.value(QStringLiteral("font")).toString(), QStringLiteral("bitmap8"));
    QCOMPARE(widget.value(QStringLiteral("radius")).toInt(), 24);
    QCOMPARE(widget.value(QStringLiteral("pixelScale")).toInt(), 0);
    QCOMPARE(widget.value(QStringLiteral("glow")).toDouble(), 1.0);
    // One bad colour voids the whole ramp: QML derives shades from the palette.
    QVERIFY(widget.value(QStringLiteral("shades")).toList().isEmpty());
    QCOMPARE(widget.value(QStringLiteral("modules")).toStringList(), moduleIds());

    // Without a background or foreground there is no theme at all.
    QJsonObject broken = fullTheme(QStringLiteral("broken"));
    broken.remove(QStringLiteral("bg"));
    QJsonObject palette = broken.value(QStringLiteral("palette")).toObject();
    palette[QStringLiteral("bg")] = QStringLiteral("not a colour");
    broken[QStringLiteral("palette")] = palette;
    QVERIFY(normalizeTheme(broken).isEmpty());
    QVERIFY(normalizeTheme(QJsonObject{}).isEmpty());
}

void TestSunReactorClient::testLegacyThemeUsesExactTableOnly()
{
    using namespace SunReactor::ThemeData;
    const QJsonObject handheld{
        {QStringLiteral("name"), QStringLiteral("Handheld")},
        {QStringLiteral("accent"), QStringLiteral("#8b1d42")},
        {QStringLiteral("secondary_accent"), QStringLiteral("#8bac0f")},
        {QStringLiteral("bg"), QStringLiteral("#141c12")},
        {QStringLiteral("fg"), QStringLiteral("#9bbc0f")},
        {QStringLiteral("text_muted"), QStringLiteral("#567648")},
    };
    const QVariantMap out = normalizeTheme(handheld);
    QCOMPARE(out.value(QStringLiteral("legacy")).toBool(), true);
    QCOMPARE(out.value(QStringLiteral("id")).toString(), QStringLiteral("handheld"));
    QCOMPARE(out.value(QStringLiteral("family")).toString(), QStringLiteral("pixel"));
    QCOMPARE(out.value(QStringLiteral("widget")).toMap().value(QStringLiteral("renderer")).toString(),
             QStringLiteral("pixel"));
    const QVariantMap palette = out.value(QStringLiteral("palette")).toMap();
    QCOMPARE(palette.value(QStringLiteral("borderActive")).toString(), QStringLiteral("#8b1d42"));
    QCOMPARE(palette.value(QStringLiteral("success")).toString(), QStringLiteral("#8bac0f"));

    // Display names fold to registry ids, accents and punctuation included.
    QCOMPARE(legacyLookup(QStringLiteral("Rosé Pine")).value(QStringLiteral("id")).toString(), QStringLiteral("rosepine"));
    QCOMPARE(legacyLookup(QStringLiteral("VFD Hi-Fi")).value(QStringLiteral("id")).toString(), QStringLiteral("vfd_hifi"));
    QCOMPARE(legacyLookup(QStringLiteral("Synthwave '84")).value(QStringLiteral("family")).toString(), QStringLiteral("pixel"));
    QCOMPARE(legacyLookup(QStringLiteral("UNIX Workstation")).value(QStringLiteral("family")).toString(),
             QStringLiteral("instrument"));
    // No substring guessing: the old heuristics turned these into skins.
    QVERIFY(legacyLookup(QStringLiteral("my gameboy theme")).isEmpty());
    QVERIFY(legacyLookup(QStringLiteral("workbench")).isEmpty());
    QJsonObject unknown = handheld;
    unknown[QStringLiteral("name")] = QStringLiteral("Handheld Deluxe");
    QCOMPARE(normalizeTheme(unknown).value(QStringLiteral("family")).toString(), QStringLiteral("instrument"));
}

void TestSunReactorClient::testOldDaemonDegradesWithoutSideEffects()
{
    QJsonObject monitor;
    monitor[QStringLiteral("logical_id")] = QStringLiteral("desk");
    monitor[QStringLiteral("backend")] = QStringLiteral("ddc");
    monitor[QStringLiteral("enabled")] = true;
    monitor[QStringLiteral("last_applied_percent")] = 40;

    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("manual_override_active")] = true;
    status[QStringLiteral("monitors")] = QJsonArray{monitor};
    status[QStringLiteral("theme")] = QJsonObject{
        {QStringLiteral("name"), QStringLiteral("Dracula")},
        {QStringLiteral("accent"), QStringLiteral("#ff79c6")},
        {QStringLiteral("secondary_accent"), QStringLiteral("#bd93f9")},
        {QStringLiteral("bg"), QStringLiteral("#282a36")},
        {QStringLiteral("fg"), QStringLiteral("#f8f8f2")},
        {QStringLiteral("text_muted"), QStringLiteral("#bfbfbf")},
    };

    serveWith([status](const QJsonObject &request) {
        const QString name = request.value(QStringLiteral("request")).toString();
        if (name == QStringLiteral("status")) {
            return statusResponse(status);
        }
        // What serde says about a variant it has never heard of.
        return errorResponse(QStringLiteral("invalid_request"),
                             QStringLiteral("failed to decode IPC request: unknown variant `%1`").arg(name));
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy errorSpy(&client, &SunReactorClient::errorOccurred);
    QSignalSpy failedSpy(&client, &SunReactorClient::themeFetchFailed);
    client.setPinnedThemeId(QStringLiteral("nord"));

    QTRY_COMPARE_WITH_TIMEOUT(client.dayCurveSupport(), SunReactorClient::CapabilityUnsupported, 3000);
    QVERIFY(client.isConnected());
    QCOMPARE(client.themeRegistry(), SunReactorClient::CapabilityUnsupported);
    QCOMPARE(client.pinnedThemeError(), QStringLiteral("unsupported"));
    QVERIFY(client.pinnedTheme().isEmpty());
    QVERIFY(client.dayCurve().isEmpty());

    // The followed theme still comes from the legacy fields.
    QCOMPARE(client.theme().value(QStringLiteral("family")).toString(), QStringLiteral("terminal"));
    QCOMPARE(client.theme().value(QStringLiteral("legacy")).toBool(), true);

    // Nothing else moved: still connected, same mode, same monitors, and the
    // rejections are not reported as errors.
    QCOMPARE(client.mode(), SunReactorClient::Manual);
    QCOMPARE(client.monitors().size(), 1);
    QCOMPARE(client.monitors().at(0).toMap().value(QStringLiteral("percent")).toInt(), 40);
    QCOMPARE(errorSpy.count(), 0);

    // Once known, the client stops asking: more polls, popup opening, a
    // theme list request and a fetch do not reach the daemon.
    const int curveRequests = countRequests(QStringLiteral("day_curve"));
    QCOMPARE(curveRequests, 1);
    // The pin was probed at most once (before the first status said "legacy").
    const int themeRequests = countRequests(QStringLiteral("get_theme"));
    QVERIFY(themeRequests <= 1);
    client.refreshThemeList();
    client.fetchTheme(QStringLiteral("braun"));
    QCOMPARE(failedSpy.count() >= 1, true);
    QCOMPARE(failedSpy.last().at(1).toString(), QStringLiteral("unsupported"));
    client.setPopupOpen(true);
    client.queryStatus();
    QTest::qWait(600);
    QCOMPARE(countRequests(QStringLiteral("day_curve")), curveRequests);
    QCOMPARE(countRequests(QStringLiteral("list_themes")), 0);
    QCOMPARE(countRequests(QStringLiteral("get_theme")), themeRequests);
    QVERIFY(client.isConnected());
}

void TestSunReactorClient::testDayCurveParsedAndNotPolled()
{
    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    serveWith([status](const QJsonObject &request) {
        if (request.value(QStringLiteral("request")).toString() == QStringLiteral("day_curve")) {
            return envelope(QStringLiteral("day_curve"), {{QStringLiteral("day_curve"), dayCurveBody()}});
        }
        return statusResponse(status);
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy curveSpy(&client, &SunReactorClient::dayCurveChanged);
    QVERIFY(curveSpy.wait(3000));
    QCOMPARE(client.dayCurveSupport(), SunReactorClient::CapabilitySupported);

    const QVariantMap curve = client.dayCurve();
    QCOMPARE(curve.value(QStringLiteral("date")).toString(), QStringLiteral("2026-03-20"));
    QCOMPARE(curve.value(QStringLiteral("stepMinutes")).toInt(), 10);
    QCOMPARE(curve.value(QStringLiteral("weatherMultiplier")).toDouble(), 0.88);
    const QVariantMap sun = curve.value(QStringLiteral("sun")).toMap();
    QCOMPARE(sun.value(QStringLiteral("solarNoon")).toLongLong(), 1773980000);
    QCOMPARE(sun.value(QStringLiteral("dusk")).toLongLong(), 0); // null stays unknown

    // The monitor without an id is dropped; bad samples are skipped.
    const QVariantList monitors = curve.value(QStringLiteral("monitors")).toList();
    QCOMPARE(monitors.size(), 1);
    const QVariantMap first = monitors.at(0).toMap();
    QCOMPARE(first.value(QStringLiteral("monitorId")).toString(), QStringLiteral("mi-monitor"));
    QCOMPARE(first.value(QStringLiteral("maxPct")).toInt(), 50);
    const QVariantList samples = first.value(QStringLiteral("samples")).toList();
    QCOMPARE(samples.size(), 3);
    QCOMPARE(samples.at(1).toMap().value(QStringLiteral("pct")).toInt(), 100);
    QCOMPARE(samples.at(1).toMap().value(QStringLiteral("pctNoWeather")).toInt(), 100);
    QCOMPARE(samples.at(2).toMap().value(QStringLiteral("pctNoWeather")).toInt(), 34);
    QCOMPARE(first.value(QStringLiteral("milestones")).toList().size(), 1);
    QCOMPARE(first.value(QStringLiteral("milestones")).toList().at(0).toMap().value(QStringLiteral("id")).toString(),
             QStringLiteral("rise_50"));

    // Status polls do not re-request the curve...
    for (int i = 0; i < 4; ++i) {
        client.queryStatus();
        QTest::qWait(60);
    }
    QTest::qWait(200);
    QCOMPARE(countRequests(QStringLiteral("day_curve")), 1);
    QVERIFY(countRequests(QStringLiteral("status")) >= 3);

    // ...but opening the popup does.
    client.setPopupOpen(true);
    QTRY_COMPARE_WITH_TIMEOUT(countRequests(QStringLiteral("day_curve")), 2, 2000);
}

void TestSunReactorClient::testDayCurveFollowsLimitChanges()
{
    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    serveWith([status](const QJsonObject &request) {
        const QString name = request.value(QStringLiteral("request")).toString();
        if (name == QStringLiteral("day_curve")) {
            return envelope(QStringLiteral("day_curve"), {{QStringLiteral("day_curve"), dayCurveBody()}});
        }
        if (name == QStringLiteral("status")) {
            return statusResponse(status);
        }
        return envelope(QStringLiteral("ack"), {{QStringLiteral("message"), QStringLiteral("ok")}});
    });

    SunReactorClient client(m_socketPath);
    QTRY_COMPARE_WITH_TIMEOUT(countRequests(QStringLiteral("day_curve")), 1, 3000);
    client.setMonitorLimits(QStringLiteral("mi-monitor"), 5, 45);
    QTRY_COMPARE_WITH_TIMEOUT(countRequests(QStringLiteral("day_curve")), 2, 3000);
    // A plain pause does not move the curve.
    client.suspend(30);
    QTest::qWait(500);
    QCOMPARE(countRequests(QStringLiteral("day_curve")), 2);
}

void TestSunReactorClient::testThemeListAndFetch()
{
    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("theme")] = fullTheme(QStringLiteral("nord"));
    serveWith([status](const QJsonObject &request) {
        const QString name = request.value(QStringLiteral("request")).toString();
        if (name == QStringLiteral("list_themes")) {
            QJsonArray themes{
                QJsonObject{{QStringLiteral("id"), QStringLiteral("nord")}, {QStringLiteral("name"), QStringLiteral("Nord")},
                            {QStringLiteral("family"), QStringLiteral("terminal")}, {QStringLiteral("bg"), QStringLiteral("#2e3440")},
                            {QStringLiteral("fg"), QStringLiteral("#d8dee9")}, {QStringLiteral("accent"), QStringLiteral("#88c0d0")},
                            {QStringLiteral("secondary_accent"), QStringLiteral("#81a1c1")}},
                QJsonObject{{QStringLiteral("id"), QStringLiteral("broken")}}, // no colours: dropped
            };
            return envelope(QStringLiteral("themes"), {{QStringLiteral("themes"), themes}});
        }
        if (name == QStringLiteral("get_theme")) {
            const QString id = request.value(QStringLiteral("id")).toString();
            if (id != QStringLiteral("nord")) {
                return errorResponse(QStringLiteral("invalid_request"), QStringLiteral("unknown theme `%1`").arg(id));
            }
            return envelope(QStringLiteral("theme"), {{QStringLiteral("theme"), fullTheme(id)}});
        }
        if (name == QStringLiteral("day_curve")) {
            return envelope(QStringLiteral("day_curve"), {{QStringLiteral("day_curve"), dayCurveBody()}});
        }
        return statusResponse(status);
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy themeSpy(&client, &SunReactorClient::themeChanged);
    QVERIFY(themeSpy.wait(2000));

    QSignalSpy listSpy(&client, &SunReactorClient::themeListChanged);
    client.refreshThemeList();
    QVERIFY(listSpy.wait(2000));
    QCOMPARE(client.themeList().size(), 1);
    QCOMPARE(client.themeList().at(0).toMap().value(QStringLiteral("secondaryAccent")).toString(),
             QStringLiteral("#81a1c1"));

    QSignalSpy fetchedSpy(&client, &SunReactorClient::themeFetched);
    QSignalSpy failedSpy(&client, &SunReactorClient::themeFetchFailed);
    client.fetchTheme(QStringLiteral("nord"));
    QVERIFY(fetchedSpy.wait(2000));
    QCOMPARE(fetchedSpy.at(0).at(0).toString(), QStringLiteral("nord"));
    QCOMPARE(fetchedSpy.at(0).at(1).toMap().value(QStringLiteral("family")).toString(), QStringLiteral("terminal"));

    // With a registry present, a rejected id is a missing theme, not an old daemon.
    client.fetchTheme(QStringLiteral("nope"));
    QVERIFY(failedSpy.wait(2000));
    QCOMPARE(failedSpy.at(0).at(1).toString(), QStringLiteral("not_found"));
    QCOMPARE(client.themeRegistry(), SunReactorClient::CapabilitySupported);
    QVERIFY(client.isConnected());
}

void TestSunReactorClient::testPinnedThemeIsKeptFetched()
{
    QJsonObject status;
    status[QStringLiteral("now_epoch_s")] = 1773998400;
    status[QStringLiteral("theme")] = fullTheme(QStringLiteral("dracula"));
    serveWith([status](const QJsonObject &request) {
        const QString name = request.value(QStringLiteral("request")).toString();
        if (name == QStringLiteral("get_theme")) {
            const QString id = request.value(QStringLiteral("id")).toString();
            if (id != QStringLiteral("nord")) {
                return errorResponse(QStringLiteral("invalid_request"), QStringLiteral("unknown theme `%1`").arg(id));
            }
            return envelope(QStringLiteral("theme"), {{QStringLiteral("theme"), fullTheme(id)}});
        }
        if (name == QStringLiteral("day_curve")) {
            return envelope(QStringLiteral("day_curve"), {{QStringLiteral("day_curve"), dayCurveBody()}});
        }
        if (name == QStringLiteral("status")) {
            return statusResponse(status);
        }
        return envelope(QStringLiteral("ack"), {{QStringLiteral("message"), QStringLiteral("ok")}});
    });

    SunReactorClient client(m_socketPath);
    client.setPinnedThemeId(QStringLiteral("nord"));
    QCOMPARE(client.pinnedThemeError(), QStringLiteral("pending"));
    QTRY_COMPARE_WITH_TIMEOUT(client.pinnedTheme().value(QStringLiteral("id")).toString(), QStringLiteral("nord"), 3000);
    QCOMPARE(client.pinnedThemeError(), QString());
    // Pinning never changes what is followed.
    QCOMPARE(client.theme().value(QStringLiteral("id")).toString(), QStringLiteral("dracula"));

    // A config reload re-fetches the pinned theme (its file may have changed).
    const int before = countRequests(QStringLiteral("get_theme"));
    client.reloadConfig();
    QTRY_VERIFY_WITH_TIMEOUT(countRequests(QStringLiteral("get_theme")) > before, 3000);

    client.setPinnedThemeId(QStringLiteral("gone"));
    QTRY_COMPARE_WITH_TIMEOUT(client.pinnedThemeError(), QStringLiteral("not_found"), 3000);
    QVERIFY(client.pinnedTheme().isEmpty());
    // A missing theme is not retried on every poll.
    const int afterMissing = countRequests(QStringLiteral("get_theme"));
    client.queryStatus();
    client.queryStatus();
    QTest::qWait(400);
    QCOMPARE(countRequests(QStringLiteral("get_theme")), afterMissing);

    client.setPinnedThemeId(QString());
    QCOMPARE(client.pinnedThemeError(), QString());
}

void TestSunReactorClient::testThemeWarningIsOptional()
{
    // The daemon only sends theme_warning when tui.theme could not be used
    // as-is (DESIGN.md section 3); older daemons never send it.
    auto warning = std::make_shared<QJsonValue>(QJsonValue::Undefined);
    serveWith([warning](const QJsonObject &request) {
        if (request.value(QStringLiteral("request")).toString() != QStringLiteral("status")) {
            return errorResponse(QStringLiteral("invalid_request"), QStringLiteral("unknown variant"));
        }
        QJsonObject status;
        status[QStringLiteral("now_epoch_s")] = 1773998400;
        status[QStringLiteral("theme")] = fullTheme(QStringLiteral("amber"));
        if (!warning->isUndefined()) {
            status[QStringLiteral("theme_warning")] = *warning;
        }
        return statusResponse(status);
    });

    SunReactorClient client(m_socketPath);
    QSignalSpy statusSpy(&client, &SunReactorClient::statusChanged);
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeWarning(), QString());

    *warning = QStringLiteral("  theme file /home/u/.config/sunreactor/themes/mine.toml is invalid: palette.bg "
                              "is missing; using the default `amber` theme instead\n");
    client.queryStatus();
    QTRY_VERIFY_WITH_TIMEOUT(!client.themeWarning().isEmpty(), 2000);
    QVERIFY(client.themeWarning().startsWith(QStringLiteral("theme file ")));
    QVERIFY2(client.themeWarning().endsWith(QStringLiteral("instead")), "surrounding whitespace is trimmed");
    // A warning is cosmetic: the theme it fell back to is still followed.
    QCOMPARE(client.theme().value(QStringLiteral("id")).toString(), QStringLiteral("amber"));

    // Omitted again (the user fixed the file) clears it.
    *warning = QJsonValue(QJsonValue::Undefined);
    client.queryStatus();
    QTRY_COMPARE_WITH_TIMEOUT(client.themeWarning(), QString(), 2000);

    // Anything but a string is treated as nothing to report.
    *warning = QJsonValue(42);
    statusSpy.clear();
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeWarning(), QString());
    *warning = QJsonValue(QJsonValue::Null);
    statusSpy.clear();
    client.queryStatus();
    QVERIFY(statusSpy.wait(2000));
    QCOMPARE(client.themeWarning(), QString());

    // Losing the daemon forgets it along with the rest of the status.
    *warning = QStringLiteral("fallback");
    client.queryStatus();
    QTRY_COMPARE_WITH_TIMEOUT(client.themeWarning(), QStringLiteral("fallback"), 2000);
    m_server->close();
    QLocalServer::removeServer(m_socketPath);
    client.queryStatus();
    QTRY_VERIFY_WITH_TIMEOUT(!client.isConnected(), 4000);
    QCOMPARE(client.themeWarning(), QString());
}

void TestSunReactorClient::testThemesFolderFollowsXdgConfigHome()
{
    const QByteArray savedXdg = qgetenv("XDG_CONFIG_HOME");
    const bool hadXdg = qEnvironmentVariableIsSet("XDG_CONFIG_HOME");
    const QString base = m_tempDir.filePath(QStringLiteral("xdg-config"));

    SunReactorClient client(m_socketPath);
    qputenv("XDG_CONFIG_HOME", base.toUtf8());
    const QString expected = base + QStringLiteral("/sunreactor/themes");
    QCOMPARE(client.themesFolder(), expected);
    QVERIFY2(!QFileInfo::exists(expected), "asking for the path creates nothing");
    QCOMPARE(client.ensureThemesFolder(), expected);
    QVERIFY(QFileInfo(expected).isDir());
    QCOMPARE(client.ensureThemesFolder(), expected); // idempotent

    // A relative XDG_CONFIG_HOME is invalid for the daemon too: no folder.
    qputenv("XDG_CONFIG_HOME", "relative/config");
    QCOMPARE(client.themesFolder(), QString());
    QCOMPARE(client.ensureThemesFolder(), QString());

    if (hadXdg) {
        qputenv("XDG_CONFIG_HOME", savedXdg);
    } else {
        qunsetenv("XDG_CONFIG_HOME");
    }
}

void TestSunReactorClient::testHarnessConfigNeedsHarness()
{
    SunReactorClient client(m_socketPath);
    qputenv("SUNREACTOR_HARNESS_CONFIG", "panelMode=ring showTemperature=false pixelScale=3 "
                                         "Appearance/moduleOrder=[\"curve\",\"displays\"] bogus =x");
    qunsetenv("SUNREACTOR_HARNESS");
    QVERIFY2(client.harnessConfig().isEmpty(), "never outside the screenshot harness");

    qputenv("SUNREACTOR_HARNESS", "1");
    const QVariantMap config = client.harnessConfig();
    QCOMPARE(config.value(QStringLiteral("panelMode")).toString(), QStringLiteral("ring"));
    QCOMPARE(config.value(QStringLiteral("showTemperature")).typeId(), QMetaType::Bool);
    QCOMPARE(config.value(QStringLiteral("showTemperature")).toBool(), false);
    QCOMPARE(config.value(QStringLiteral("pixelScale")).toInt(), 3);
    QCOMPARE(config.value(QStringLiteral("moduleOrder")).toStringList(),
             (QStringList{QStringLiteral("curve"), QStringLiteral("displays")}));
    QCOMPARE(config.size(), 4);
    qunsetenv("SUNREACTOR_HARNESS");
    qunsetenv("SUNREACTOR_HARNESS_CONFIG");
}

QTEST_MAIN(TestSunReactorClient)
#include "test_sunreactorclient.moc"
