#include "sunreactorclient_private.h"

#include <QDateTime>
#include <QJsonArray>

#include "themenormalize.h"

using namespace SunReactorClientPrivate;

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
    const QString previousTemperatureUnit = m_temperatureUnit;
    const QString previousLocationCity = m_locationCity;
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
    m_themeWarning = statusObj.value(QStringLiteral("theme_warning")).toString().trimmed().left(kMaxThemeWarningLength);
    const QString rawTempUnit = statusObj.value(QStringLiteral("temperature_unit")).toString().trimmed().toLower();
    m_temperatureUnit = (rawTempUnit == QLatin1String("fahrenheit"))
        ? QStringLiteral("fahrenheit")
        : QStringLiteral("celsius");
    m_locationCity = displayString(statusObj.value(QStringLiteral("location_city")));

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
        || previousTemperatureUnit != m_temperatureUnit
        || previousLocationCity != m_locationCity
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
    const bool needsPinnedFetch = (m_pinnedTheme.isEmpty() && m_pinnedThemeError.isEmpty())
        || m_pinnedThemeError == QLatin1String("pending")
        || m_pinnedThemeError == QLatin1String("offline");
    if (!m_pinnedThemeId.isEmpty() && needsPinnedFetch
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
        if (!m_pinnedThemeId.isEmpty() && m_pinnedThemeError == QLatin1String("unsupported")) {
            setPinned(m_pinnedTheme, QStringLiteral("pending"));
        }
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
        // Monitor ids are machine keys: preserve them for QML lookups and
        // reject only values too large to send back in a bounded request.
        const QString id = monitor.value(QStringLiteral("monitor_id")).toString();
        if (!validRequestIdentifier(id)) {
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
        m_weatherSourceKind.clear();
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
    m_weatherSourceKind = displayString(weather.value(QStringLiteral("source_kind")));
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
        // QML sends this value back to the daemon, so it must not use the
        // display-only 64-character bound applied to descriptive strings.
        const QString logicalId = monitor.value(QStringLiteral("logical_id")).toString();
        if (!validRequestIdentifier(logicalId)) {
            continue;
        }
        map[QStringLiteral("logicalId")] = logicalId;
        map[QStringLiteral("backend")] = displayString(monitor.value(QStringLiteral("backend")));
        map[QStringLiteral("topology")] = displayString(monitor.value(QStringLiteral("topology")));
        map[QStringLiteral("enabled")] = enabled;
        map[QStringLiteral("appliedPercent")] = applied;
        map[QStringLiteral("minPct")] = minPct;
        map[QStringLiteral("maxPct")] = maxPct;
        map[QStringLiteral("hasOverride")] = overridePercent != UNKNOWN_PERCENT;
        map[QStringLiteral("overridePercent")] = overridePercent;
        // What a slider should show: the override the user set, otherwise the
        // value the daemon last wrote, otherwise nothing.
        map[QStringLiteral("percent")] = overridePercent != UNKNOWN_PERCENT ? overridePercent : applied;
        map[QStringLiteral("unreachable")] = unreachable;
        map[QStringLiteral("backoffRemainingS")] = unreachable ? backoffUntil - m_nowEpochS : 0;

        map[QStringLiteral("contested")] = monitor.value(QStringLiteral("contested")).toBool(false);
        map[QStringLiteral("contestedUntilS")] = optionalEpoch(monitor, "contested_until_epoch_s");

        const QJsonValue writesValue = monitor.value(QStringLiteral("writes"));
        if (writesValue.isObject()) {
            const QJsonObject writes = writesValue.toObject();
            map[QStringLiteral("writesDeferred")] = writes.value(QStringLiteral("deferred")).toBool(false);
            map[QStringLiteral("writesWindowUsed")] = qMax(0, writes.value(QStringLiteral("window_used")).toInt(0));
            map[QStringLiteral("writesWindowMax")] = qMax(0, writes.value(QStringLiteral("window_max")).toInt(0));
            map[QStringLiteral("writesTotal")] = qMax<qint64>(0, writes.value(QStringLiteral("writes_total")).toInteger(0));
        } else {
            map[QStringLiteral("writesDeferred")] = false;
            map[QStringLiteral("writesWindowUsed")] = 0;
            map[QStringLiteral("writesWindowMax")] = 0;
            map[QStringLiteral("writesTotal")] = static_cast<qint64>(0);
        }

        const QJsonValue externalHoldValue = monitor.value(QStringLiteral("external_hold"));
        if (externalHoldValue.isObject()) {
            const QJsonObject externalHold = externalHoldValue.toObject();
            const qint64 remaining = optionalEpoch(externalHold, "remaining_s");
            const int observed = optionalPercent(externalHold, "observed_percent");
            if (remaining > 0 && observed != UNKNOWN_PERCENT) {
                QVariantMap hold;
                hold[QStringLiteral("remainingS")] = remaining;
                hold[QStringLiteral("observedPercent")] = observed;
                map[QStringLiteral("externalHold")] = hold;
            }
        }

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
    const QVariantList previousMonitors = m_monitors;
    const bool previousWeatherEnabled = m_weatherEnabled;
    const bool previousWeatherStale = m_weatherStale;
    const bool previousHasWeather = m_hasWeatherReading;
    const QString previousWeatherState = m_weatherState;
    const QString previousWeatherCondition = m_weatherCondition;
    const QString previousWeatherDescription = m_weatherDescription;
    const QString previousWeatherDayPhase = m_weatherDayPhase;
    const int previousCloudCover = m_cloudCoverPercent;
    const bool previousHasTemperature = m_hasTemperature;
    const double previousTemperature = m_temperatureC;
    const QString previousTemperatureUnit = m_temperatureUnit;
    const QString previousLocationCity = m_locationCity;
    const double previousWeatherMultiplier = m_weatherMultiplier;
    const QVariantList previousForecast = m_forecast;
    const QVariantMap previousDetails = m_weatherDetails;
    const QString previousThemeName = m_themeName;
    const QString previousThemeAccent = m_themeAccent;
    const QString previousThemeSecondaryAccent = m_themeSecondaryAccent;
    const QString previousThemeBg = m_themeBg;
    const QString previousThemeFg = m_themeFg;
    const QString previousThemeTextMuted = m_themeTextMuted;
    const QString previousThemeWarning = m_themeWarning;
    const bool previousDaylight = isDaylight();
    const bool previousWeatherNight = weatherIsNight();

    m_isSuspended = false;
    m_isOverrideActive = false;
    m_isDesktopIdleDimmed = false;
    m_hasSolarElevation = false;
    m_solarElevation = 0.0;
    m_nowEpochS = 0;
    m_sunriseEpochS = 0;
    m_sunsetEpochS = 0;
    m_suspendUntilEpochS = 0;
    m_globalOverrideUntilEpochS = 0;
    m_perMonitorOverrideUntilEpochS = 0;
    m_globalPercent = UNKNOWN_PERCENT;
    m_cloudDimmingPercent = UNKNOWN_PERCENT;
    m_monitors.clear();
    m_weatherEnabled = false;
    m_weatherStale = false;
    m_hasWeatherReading = false;
    m_weatherState.clear();
    m_weatherCondition.clear();
    m_weatherDescription.clear();
    m_weatherDayPhase.clear();
    m_weatherSourceKind.clear();
    m_cloudCoverPercent = UNKNOWN_PERCENT;
    m_hasTemperature = false;
    m_temperatureC = 0.0;
    m_temperatureUnit = QStringLiteral("celsius");
    m_locationCity.clear();
    m_weatherMultiplier = 1.0;
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
    if (previousNow != m_nowEpochS) {
        Q_EMIT nowChanged();
    }
    if (previousSuspended != m_isSuspended || previousOverride != m_isOverrideActive
        || previousIdleDimmed != m_isDesktopIdleDimmed || previousHasElevation != m_hasSolarElevation
        || previousElevation != m_solarElevation || previousSunrise != m_sunriseEpochS
        || previousSunset != m_sunsetEpochS || previousSuspendUntil != m_suspendUntilEpochS
        || previousGlobalOverrideUntil != m_globalOverrideUntilEpochS
        || previousMonitorOverrideUntil != m_perMonitorOverrideUntilEpochS
        || previousGlobalPercent != m_globalPercent || previousCloudDimming != m_cloudDimmingPercent
        || previousWeatherEnabled != m_weatherEnabled || previousWeatherStale != m_weatherStale
        || previousHasWeather != m_hasWeatherReading || previousWeatherState != m_weatherState
        || previousWeatherCondition != m_weatherCondition
        || previousWeatherDescription != m_weatherDescription
        || previousWeatherDayPhase != m_weatherDayPhase || previousCloudCover != m_cloudCoverPercent
        || previousHasTemperature != m_hasTemperature || previousTemperature != m_temperatureC
        || previousTemperatureUnit != m_temperatureUnit
        || previousLocationCity != m_locationCity
        || previousWeatherMultiplier != m_weatherMultiplier || previousThemeName != m_themeName
        || previousThemeAccent != m_themeAccent || previousThemeSecondaryAccent != m_themeSecondaryAccent
        || previousThemeBg != m_themeBg || previousThemeFg != m_themeFg
        || previousThemeTextMuted != m_themeTextMuted || previousThemeWarning != m_themeWarning
        || previousDaylight != isDaylight() || previousWeatherNight != weatherIsNight()) {
        Q_EMIT statusChanged();
    }
}

