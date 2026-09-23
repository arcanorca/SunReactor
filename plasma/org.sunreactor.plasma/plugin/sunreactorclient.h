#pragma once

#include <QByteArray>
#include <QJsonArray>
#include <QJsonObject>
#include <QLocalSocket>
#include <QObject>
#include <QQueue>
#include <QString>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>
#include <QtQml/qqmlregistration.h>

#include "sunreactortypes.h"

/*!
 * Transport for the SunReactor daemon's local control socket.
 *
 * The client deliberately exposes data only: enums, epochs, numbers and
 * tokens. Every user-visible string - mode names, durations, temperatures,
 * condition names - is built in QML, where it can be translated and formatted
 * for the user's locale. Values the daemon does not report stay unknown
 * (\c UNKNOWN_PERCENT, an invalid QVariant, or a zero epoch); the client never
 * substitutes a plausible number.
 */
class SunReactorClient : public QObject
{
    Q_OBJECT
    QML_NAMED_ELEMENT(SunReactorClient)

public:
    /*! What the daemon is doing right now, in order of precedence. */
    enum Mode {
        Offline,    //!< No answer from the control socket.
        Paused,     //!< Automatic brightness suspended by the user.
        Manual,     //!< A manual override is in effect.
        IdleDimmed, //!< The desktop is idle and the daemon dimmed the screens.
        Automatic,  //!< Following the solar (and weather) policy.
    };
    Q_ENUM(Mode)

private:
    Q_PROPERTY(bool isConnected READ isConnected NOTIFY connectionChanged)
    Q_PROPERTY(bool isBusy READ isBusy NOTIFY busyChanged)
    Q_PROPERTY(QString lastError READ lastError NOTIFY errorOccurred)
    Q_PROPERTY(QString socketPath READ socketPath WRITE setSocketPath NOTIFY socketPathChanged)
    Q_PROPERTY(bool popupOpen READ popupOpen WRITE setPopupOpen NOTIFY popupOpenChanged)

    Q_PROPERTY(Mode mode READ mode NOTIFY statusChanged)
    Q_PROPERTY(bool isSuspended READ isSuspended NOTIFY statusChanged)
    Q_PROPERTY(bool isOverrideActive READ isOverrideActive NOTIFY statusChanged)
    Q_PROPERTY(qint64 suspendUntilEpochS READ suspendUntilEpochS NOTIFY statusChanged)
    Q_PROPERTY(qint64 overrideUntilEpochS READ overrideUntilEpochS NOTIFY statusChanged)

    /*! Mean applied brightness, or \c UNKNOWN_PERCENT while nothing is known. */
    Q_PROPERTY(int globalPercent READ globalPercent NOTIFY statusChanged)
    Q_PROPERTY(QVariantList monitors READ monitors NOTIFY statusChanged)

    Q_PROPERTY(qint64 nowEpochS READ nowEpochS NOTIFY statusChanged)
    Q_PROPERTY(qint64 sunriseEpochS READ sunriseEpochS NOTIFY statusChanged)
    Q_PROPERTY(qint64 sunsetEpochS READ sunsetEpochS NOTIFY statusChanged)
    /*! Midpoint between sunrise and sunset; zero when either is unknown. */
    Q_PROPERTY(qint64 solarNoonEpochS READ solarNoonEpochS NOTIFY statusChanged)
    Q_PROPERTY(bool isDaylight READ isDaylight NOTIFY statusChanged)
    Q_PROPERTY(bool hasSolarElevation READ hasSolarElevation NOTIFY statusChanged)
    /*! Sun height above the horizon in degrees; meaningless without the flag. */
    Q_PROPERTY(double solarElevation READ solarElevation NOTIFY statusChanged)

    Q_PROPERTY(bool weatherEnabled READ weatherEnabled NOTIFY statusChanged)
    /*! Daemon-side weather state token, e.g. "ready", "stale", "no_api_key". */
    Q_PROPERTY(QString weatherState READ weatherState NOTIFY statusChanged)
    Q_PROPERTY(bool weatherStale READ weatherStale NOTIFY statusChanged)
    Q_PROPERTY(bool hasWeatherReading READ hasWeatherReading NOTIFY statusChanged)
    /*! Condition token, e.g. "clear", "partly_cloudy", "heavy_rain". */
    Q_PROPERTY(QString weatherCondition READ weatherCondition NOTIFY statusChanged)
    Q_PROPERTY(bool weatherIsNight READ weatherIsNight NOTIFY statusChanged)
    Q_PROPERTY(int cloudCoverPercent READ cloudCoverPercent NOTIFY statusChanged)
    Q_PROPERTY(bool hasTemperature READ hasTemperature NOTIFY statusChanged)
    Q_PROPERTY(double temperatureC READ temperatureC NOTIFY statusChanged)
    /*! Bounded weather modifier; 1.0 means "no effect on the target". */
    Q_PROPERTY(double weatherMultiplier READ weatherMultiplier NOTIFY statusChanged)
    /*! Upcoming forecast intervals, oldest first, as maps for QML. */
    Q_PROPERTY(QVariantList forecast READ forecast NOTIFY statusChanged)
    /*! Secondary readings; keys are absent when the provider did not send them. */
    Q_PROPERTY(QVariantMap weatherDetails READ weatherDetails NOTIFY statusChanged)

public:
    explicit SunReactorClient(QObject *parent = nullptr);
    explicit SunReactorClient(const QString &customSocketPath, QObject *parent = nullptr);
    ~SunReactorClient() override;

    [[nodiscard]] bool isConnected() const { return m_isConnected; }
    [[nodiscard]] bool isBusy() const { return m_isBusy; }
    [[nodiscard]] QString lastError() const { return m_lastError; }
    [[nodiscard]] QString socketPath() const { return m_customSocketPath; }
    [[nodiscard]] bool popupOpen() const { return m_popupOpen; }

    [[nodiscard]] Mode mode() const;
    [[nodiscard]] bool isSuspended() const { return m_isSuspended; }
    [[nodiscard]] bool isOverrideActive() const { return m_isOverrideActive; }
    [[nodiscard]] qint64 suspendUntilEpochS() const { return m_suspendUntilEpochS; }
    [[nodiscard]] qint64 overrideUntilEpochS() const;

    [[nodiscard]] int globalPercent() const { return m_globalPercent; }
    [[nodiscard]] QVariantList monitors() const { return m_monitors; }

    [[nodiscard]] qint64 nowEpochS() const { return m_nowEpochS; }
    [[nodiscard]] qint64 sunriseEpochS() const { return m_sunriseEpochS; }
    [[nodiscard]] qint64 sunsetEpochS() const { return m_sunsetEpochS; }
    [[nodiscard]] qint64 solarNoonEpochS() const;
    [[nodiscard]] bool isDaylight() const;
    [[nodiscard]] bool hasSolarElevation() const { return m_hasSolarElevation; }
    [[nodiscard]] double solarElevation() const { return m_solarElevation; }

    [[nodiscard]] bool weatherEnabled() const { return m_weatherEnabled; }
    [[nodiscard]] QString weatherState() const { return m_weatherState; }
    [[nodiscard]] bool weatherStale() const { return m_weatherStale; }
    [[nodiscard]] bool hasWeatherReading() const { return m_hasWeatherReading; }
    [[nodiscard]] QString weatherCondition() const { return m_weatherCondition; }
    [[nodiscard]] bool weatherIsNight() const;
    [[nodiscard]] int cloudCoverPercent() const { return m_cloudCoverPercent; }
    [[nodiscard]] bool hasTemperature() const { return m_hasTemperature; }
    [[nodiscard]] double temperatureC() const { return m_temperatureC; }
    [[nodiscard]] double weatherMultiplier() const { return m_weatherMultiplier; }
    [[nodiscard]] QVariantList forecast() const { return m_forecast; }
    [[nodiscard]] QVariantMap weatherDetails() const { return m_weatherDetails; }

    void setSocketPath(const QString &path);
    void setPopupOpen(bool open);

public Q_SLOTS:
    void queryStatus();
    void setGlobalOverride(int percent, int durationMinutes);
    void clearGlobalOverride();
    void setMonitorOverride(const QString &monitorId, int percent, int durationMinutes);
    void clearMonitorOverride(const QString &monitorId);
    void clearAllOverrides();
    void suspend(int minutes);
    void resume();
    void runOnce(bool force);
    void refreshWeather();
    void reloadConfig();
    void ping();
    /*! Opens `sunreactorctl tui` in the session's terminal emulator. */
    void openTerminal();

Q_SIGNALS:
    void statusChanged();
    void connectionChanged(bool connected);
    void errorOccurred(const QString &errorMessage);
    void socketPathChanged();
    void busyChanged();
    void popupOpenChanged();

private Q_SLOTS:
    void onSocketConnected();
    void onSocketReadyRead();
    void onSocketDisconnected();
    void onSocketError(QLocalSocket::LocalSocketError socketError);
    void onRequestTimeout();
    void onPollTimerTriggered();

private:
    [[nodiscard]] QJsonObject request(const QString &name) const;
    void enqueueRequest(const QJsonObject &requestObj);
    void processNextRequest();
    void sendInFlightRequest();
    void handleResponse(const QJsonObject &responseObj);
    void parseStatus(const QJsonObject &statusObj);
    void parseWeather(const QJsonObject &statusObj);
    void parseMonitors(const QJsonObject &statusObj);
    void parseForecast(const QJsonArray &entries);
    void parseDetails(const QJsonObject &details);
    void clearStatus();
    void setConnected(bool connected);
    void setBusy(bool busy);
    void fail(const QString &message);

    [[nodiscard]] QString defaultSocketPath() const;
    [[nodiscard]] QString resolvedSocketPath() const;

    QLocalSocket *m_socket = nullptr;
    QTimer *m_pollTimer = nullptr;
    QTimer *m_watchdogTimer = nullptr;

    QQueue<QJsonObject> m_requestQueue;
    QJsonObject m_inFlightRequest;
    QByteArray m_readBuffer;

    QString m_customSocketPath;
    QString m_lastError;

    bool m_isConnected = false;
    bool m_isBusy = false;
    bool m_popupOpen = false;

    bool m_isSuspended = false;
    bool m_isOverrideActive = false;
    bool m_isDesktopIdleDimmed = false;

    int m_globalPercent = SunReactor::UNKNOWN_PERCENT;
    QVariantList m_monitors;

    bool m_hasSolarElevation = false;
    double m_solarElevation = 0.0;
    qint64 m_nowEpochS = 0;
    qint64 m_sunriseEpochS = 0;
    qint64 m_sunsetEpochS = 0;
    qint64 m_suspendUntilEpochS = 0;
    qint64 m_globalOverrideUntilEpochS = 0;
    qint64 m_perMonitorOverrideUntilEpochS = 0;

    bool m_weatherEnabled = false;
    bool m_weatherStale = false;
    bool m_hasWeatherReading = false;
    QString m_weatherState;
    QString m_weatherCondition;
    QString m_weatherDayPhase;
    int m_cloudCoverPercent = SunReactor::UNKNOWN_PERCENT;
    bool m_hasTemperature = false;
    double m_temperatureC = 0.0;
    double m_weatherMultiplier = 1.0;
    QVariantList m_forecast;
    QVariantMap m_weatherDetails;
};
