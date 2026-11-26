#include <QQmlProperty>
#include <cmath>
#include <QCoreApplication>
#include <QCursor>
#include <QFont>
#include <QGuiApplication>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalServer>
#include <QLocalSocket>
#include <QQmlComponent>
#include <QQmlContext>
#include <QQmlEngine>
#include <QQuickItem>
#include <QQuickView>
#include <QTemporaryDir>
#include <QSignalSpy>
#include <QTest>

class MockDaemon : public QObject
{
    Q_OBJECT

public:
    explicit MockDaemon(const QString &socketPath, QObject *parent = nullptr)
        : QObject(parent)
    {
        QLocalServer::removeServer(socketPath);
        m_listening = m_server.listen(socketPath);
        connect(&m_server, &QLocalServer::newConnection, this, [this]() {
            while (QLocalSocket *sock = m_server.nextPendingConnection()) {
                connect(sock, &QLocalSocket::readyRead, this, [this, sock]() {
                    const QByteArray raw = sock->readAll();
                    const QJsonObject req = QJsonDocument::fromJson(raw).object();
                    const QString name = req.value(QStringLiteral("request")).toString();
                    QJsonObject resp;
                    resp[QStringLiteral("version")] = 1;
                    if (name == QStringLiteral("status")) {
                        resp[QStringLiteral("response")] = QStringLiteral("status");
                        resp[QStringLiteral("status")] = statusPayload();
                    } else if (name == QStringLiteral("day_curve")) {
                        resp[QStringLiteral("response")] = QStringLiteral("day_curve");
                        resp[QStringLiteral("day_curve")] = dayCurvePayload();
                    } else {
                        resp[QStringLiteral("response")] = QStringLiteral("ack");
                    }
                    sock->write(QJsonDocument(resp).toJson(QJsonDocument::Compact) + '\n');
                    sock->flush();
                });
                connect(sock, &QLocalSocket::disconnected, sock, &QObject::deleteLater);
            }
        });
    }

    bool isListening() const { return m_listening; }

    void setThemeSpec(const QString &id, const QString &renderer, const QString &font)
    {
        m_themeId = id;
        m_renderer = renderer;
        m_font = font;
    }

    void setM1Percent(int pct)
    {
        m_m1Percent = pct;
    }

private:
    QJsonObject statusPayload() const
    {
        QJsonObject palette;
        palette[QStringLiteral("bg")] = QStringLiteral("#0a0a12");
        palette[QStringLiteral("fg")] = QStringLiteral("#fcee0a");
        palette[QStringLiteral("accent")] = QStringLiteral("#00f0ff");
        palette[QStringLiteral("secondary_accent")] = QStringLiteral("#ff003c");
        palette[QStringLiteral("border_active")] = QStringLiteral("#00f0ff");
        palette[QStringLiteral("border_inactive")] = QStringLiteral("#2a2a40");
        palette[QStringLiteral("text_muted")] = QStringLiteral("#7f849c");
        palette[QStringLiteral("success")] = QStringLiteral("#00ff88");
        palette[QStringLiteral("warning")] = QStringLiteral("#fcee0a");
        palette[QStringLiteral("error")] = QStringLiteral("#ff003c");

        QJsonObject widget;
        widget[QStringLiteral("renderer")] = m_renderer;
        widget[QStringLiteral("font")] = m_font;
        widget[QStringLiteral("density")] = QStringLiteral("comfortable");
        widget[QStringLiteral("border")] = QStringLiteral("line");
        widget[QStringLiteral("radius")] = 0;
        widget[QStringLiteral("glow")] = 0.5;
        widget[QStringLiteral("shades")] = QJsonArray{
            QStringLiteral("#0a0a12"), QStringLiteral("#2a2a40"),
            QStringLiteral("#7f849c"), QStringLiteral("#fcee0a")};
        widget[QStringLiteral("modules")] = QJsonArray{
            QStringLiteral("telemetry"), QStringLiteral("curve"), QStringLiteral("displays"),
            QStringLiteral("automation"), QStringLiteral("weather"), QStringLiteral("forecast"),
            QStringLiteral("suntimes")};

        QJsonObject theme;
        theme[QStringLiteral("id")] = m_themeId;
        theme[QStringLiteral("name")] = m_themeId;
        theme[QStringLiteral("family")] = m_renderer;
        theme[QStringLiteral("palette")] = palette;
        theme[QStringLiteral("widget")] = widget;

        QJsonObject m1;
        m1[QStringLiteral("logical_id")] = QStringLiteral("DP-3");
        m1[QStringLiteral("enabled")] = true;
        m1[QStringLiteral("last_applied_percent")] = m_m1Percent;
        m1[QStringLiteral("min_pct")] = 15;
        m1[QStringLiteral("max_pct")] = 85;

        QJsonObject m2;
        m2[QStringLiteral("logical_id")] = QStringLiteral("DP-1");
        m2[QStringLiteral("enabled")] = true;
        m2[QStringLiteral("last_applied_percent")] = 60;
        m2[QStringLiteral("min_pct")] = 20;
        m2[QStringLiteral("max_pct")] = 90;

        QJsonObject details;
        details[QStringLiteral("feels_like")] = 14.0;
        details[QStringLiteral("humidity_percent")] = 62;
        details[QStringLiteral("wind_speed_mps")] = 3.4;
        QJsonObject airQuality;
        airQuality[QStringLiteral("us_aqi")] = 42;
        details[QStringLiteral("air_quality")] = airQuality;

        QJsonArray forecast;
        for (int i = 0; i < 5; ++i) {
            QJsonObject pt;
            pt[QStringLiteral("dt_epoch_s")] = 1700046000 + (i + 1) * 3600;
            pt[QStringLiteral("temperature")] = 15.0 - i;
            pt[QStringLiteral("condition")] = QStringLiteral("partly_cloudy");
            pt[QStringLiteral("day_phase")] = QStringLiteral("day");
            pt[QStringLiteral("cloud_cover_percent")] = 40 + i * 10;
            forecast.append(pt);
        }

        QJsonObject weather;
        weather[QStringLiteral("enabled")] = true;
        weather[QStringLiteral("state")] = QStringLiteral("ok");
        weather[QStringLiteral("condition")] = QStringLiteral("partly_cloudy");
        weather[QStringLiteral("day_phase")] = QStringLiteral("day");
        weather[QStringLiteral("temperature")] = 15.2;
        weather[QStringLiteral("multiplier")] = 0.88;
        weather[QStringLiteral("cloud_cover_percent")] = 45;
        weather[QStringLiteral("details")] = details;
        weather[QStringLiteral("forecast")] = forecast;

        QJsonObject status;
        status[QStringLiteral("now_epoch_s")] = 1700046000;
        status[QStringLiteral("sunrise_epoch_s")] = 1700024400;
        status[QStringLiteral("sunset_epoch_s")] = 1700060400;
        status[QStringLiteral("theme")] = theme;
        status[QStringLiteral("monitors")] = QJsonArray{m1, m2};
        status[QStringLiteral("weather")] = weather;
        return status;
    }

    static QJsonObject dayCurvePayload()
    {
        QJsonArray samples;
        const qint64 t0 = 1700002800;
        for (int i = 0; i < 96; ++i) {
            QJsonObject s;
            s[QStringLiteral("t")] = t0 + i * 900;
            const int pct = (i >= 24 && i <= 64) ? 55 : 15;
            s[QStringLiteral("pct")] = pct;
            s[QStringLiteral("pct_no_weather")] = pct + 10;
            samples.append(s);
        }

        QJsonObject m1;
        m1[QStringLiteral("monitor_id")] = QStringLiteral("DP-3");
        m1[QStringLiteral("min_pct")] = 15;
        m1[QStringLiteral("max_pct")] = 85;
        m1[QStringLiteral("samples")] = samples;

        QJsonObject m2;
        m2[QStringLiteral("monitor_id")] = QStringLiteral("DP-1");
        m2[QStringLiteral("min_pct")] = 20;
        m2[QStringLiteral("max_pct")] = 90;
        m2[QStringLiteral("samples")] = samples;

        QJsonObject sun;
        sun[QStringLiteral("sunrise")] = 1700024400;
        sun[QStringLiteral("solar_noon")] = 1700042400;
        sun[QStringLiteral("sunset")] = 1700060400;

        QJsonObject curve;
        curve[QStringLiteral("date")] = QStringLiteral("2026-11-17");
        curve[QStringLiteral("step_minutes")] = 15;
        curve[QStringLiteral("sun")] = sun;
        curve[QStringLiteral("monitors")] = QJsonArray{m1, m2};
        return curve;
    }

    QLocalServer m_server;
    bool m_listening = false;
    QString m_themeId = QStringLiteral("cyberpunk");
    QString m_renderer = QStringLiteral("vector");
    QString m_font = QStringLiteral("sans");
    int m_m1Percent = 55;
};

class I18nStub : public QObject
{
    Q_OBJECT
public:
    using QObject::QObject;
    Q_INVOKABLE QString i18nd(const QString &, const QString &text,
                              const QVariant &a1 = {}, const QVariant &a2 = {},
                              const QVariant &a3 = {}) const
    {
        return formatArgs(text, a1, a2, a3);
    }
    Q_INVOKABLE QString i18ndc(const QString &, const QString &, const QString &text,
                               const QVariant &a1 = {}, const QVariant &a2 = {},
                               const QVariant &a3 = {}) const
    {
        return formatArgs(text, a1, a2, a3);
    }
    Q_INVOKABLE QString i18ndp(const QString &, const QString &singular, const QString &plural,
                               int n, const QVariant &a1 = {}, const QVariant &a2 = {}) const
    {
        return formatArgs(n == 1 ? singular : plural, n, a1, a2);
    }

private:
    static QString formatArgs(const QString &text, const QVariant &a1,
                              const QVariant &a2, const QVariant &a3)
    {
        QString out = text;
        if (a1.isValid()) {
            out.replace(QStringLiteral("%1"), a1.toString());
        }
        if (a2.isValid()) {
            out.replace(QStringLiteral("%2"), a2.toString());
        }
        if (a3.isValid()) {
            out.replace(QStringLiteral("%3"), a3.toString());
        }
        return out;
    }
};

class TestCyberpunkLayout : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void cyberpunkLightCycleSettlesOnResize();
    void cyberpunkPageIsStableAtFractionalScale();
    void cyberpunkHoverScrubDoesNotOscillate();
    void weatherMetersUseArrowCursor();
    void allSkinsSettleAndScrubWithoutOscillation();
    void rangeDrawerPersistsAcrossMonitorsChanged();
    void zeroPercentLeftEdgeReachableOnPixelSliders();
    void instrumentPausedArrowOpensMenuAndTracksActivePopupMenus();
    void tabBarsStepWithLeftRightKeys();
};

void TestCyberpunkLayout::cyberpunkLightCycleSettlesOnResize()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);
    view.resize(340, 420);

    const QString uiDir = QStringLiteral(PLASMA_UI_DIR);
    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.kde.plasma.components as PlasmaComponents3
        import org.kde.plasma.extras as PlasmaExtras
        import org.kde.kirigami as Kirigami
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod
        import "components" as Comp
        import "components/tabs" as Tabs
        import "skins/cyberpunk" as Cyber

        PlasmaExtras.Representation {
            id: root
            property alias client: client
            property alias themeObj: theme
            property alias settingsObj: settings
            property alias lightCycle: cycleLoader.item
            readonly property real edgeInset: Math.round(Kirigami.Units.smallSpacing * theme.scaleFactor)

            Layout.minimumWidth: theme.gridUnit * 16
            Layout.preferredWidth: theme.gridUnit * 19
            Layout.maximumWidth: theme.gridUnit * 40
            width: theme.gridUnit * 19
            height: 560

            SunReactorClient {
                id: client
                socketPath: testSocketPath
            }
            Engine.Theme {
                id: theme
                spec: client.theme
                source: "follow"
                overrides: ({ uiScale: 115 })
            }
            Mod.ModuleSettings {
                id: settings
            }
            Binding {
                target: theme
                property: "gridContentWidth"
                value: pages.width - 2 * root.edgeInset - theme.gridUnit
            }

            contentItem: ColumnLayout {
                spacing: 0
                ColumnLayout {
                    id: headerColumn
                    Layout.fillWidth: true
                    Layout.leftMargin: root.edgeInset
                    Layout.rightMargin: root.edgeInset
                    spacing: 0
                    Loader {
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Telemetry.qml"), {
                            theme: root.themeObj,
                            client: root.client,
                            settings: root.settingsObj,
                        })
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        Loader {
                            Layout.fillWidth: true
                            Component.onCompleted: setSource(Qt.resolvedUrl("components/tabs/InstrumentTabs.qml"), {
                                theme: root.themeObj,
                                tabs: [{ id: "brightness", text: "Brightness", hint: "" },
                                       { id: "weather", text: "Weather", hint: "" }],
                                current: "brightness",
                            })
                        }
                    }
                }
                Item {
                    id: pages
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    Layout.topMargin: theme.sectionSpacing
                    PlasmaComponents3.ScrollView {
                        id: panel
                        anchors.fill: parent
                        PlasmaComponents3.ScrollBar.horizontal.visible: false
                        ColumnLayout {
                            id: pageColumn
                            x: root.edgeInset
                            width: panel.width - 2 * root.edgeInset - theme.gridUnit
                            spacing: theme.sectionSpacing
                            Loader {
                                id: cycleLoader
                                Layout.fillWidth: true
                                Layout.maximumWidth: parent ? parent.width : 0
                                Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/LightCycle.qml"), {
                                    theme: root.themeObj,
                                    client: root.client,
                                    settings: root.settingsObj,
                                })
                            }
                            Loader {
                                id: displaysLoader
                                Layout.fillWidth: true
                                Layout.maximumWidth: parent ? parent.width : 0
                                Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Displays.qml"), {
                                    theme: root.themeObj,
                                    client: root.client,
                                    settings: root.settingsObj,
                                })
                            }
                            Loader {
                                id: autoLoader
                                Layout.fillWidth: true
                                Layout.maximumWidth: parent ? parent.width : 0
                                Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Automation.qml"), {
                                    theme: root.themeObj,
                                    client: root.client,
                                    settings: root.settingsObj,
                                })
                            }
                        }
                    }
                }
            }
            footer: Comp.PopupFooter {
                client: root.client
                theme: root.themeObj
            }
        }
    )";

    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(uiDir + QStringLiteral("/TestStage.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));

    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    auto *rootItem = qobject_cast<QQuickItem *>(rootObj);
    QVERIFY(rootItem != nullptr);

    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *client = rootObj->property("client").value<QObject *>();
    QVERIFY(client != nullptr);
    client->setProperty("socketPath", sockPath);
    QMetaObject::invokeMethod(client, "queryStatus");

    QObject *cycle = rootObj->property("lightCycle").value<QObject *>();
    QVERIFY(cycle != nullptr);
    for (int i = 0; i < 100 && !cycle->property("available").toBool(); ++i) {
        QTest::qWait(20);
    }
    if (!cycle->property("available").toBool()) {
        qWarning() << "sockPath:" << sockPath
                   << "clientSocketPath:" << client->property("socketPath")
                   << "connected:" << client->property("isConnected")
                   << "lastError:" << client->property("lastError")
                   << "dayCurve:" << client->property("dayCurve");
    }
    QVERIFY(cycle->property("available").toBool());

    QQuickItem *frame = nullptr;
    for (QQuickItem *child : qobject_cast<QQuickItem *>(cycle)->childItems()) {
        if (child->property("hatched").isValid()) {
            frame = child;
            break;
        }
    }
    QVERIFY(frame != nullptr);

    // Let initial layout pass complete after data arrival before measuring stability.
    for (int turn = 0; turn < 10; ++turn) {
        view.requestUpdate();
        QCoreApplication::processEvents();
        QTest::qWait(8);
    }

    int implicitWidthChanges = 0;
    int widthChanges = 0;
    int allDescendantWidthChanges = 0;
    connect(frame, &QQuickItem::implicitWidthChanged, this, [&implicitWidthChanges]() {
        ++implicitWidthChanges;
    });
    connect(frame, &QQuickItem::widthChanged, this, [&widthChanges]() {
        ++widthChanges;
    });
    for (QQuickItem *desc : frame->findChildren<QQuickItem *>()) {
        connect(desc, &QQuickItem::widthChanged, this, [&allDescendantWidthChanges]() {
            ++allDescendantWidthChanges;
        });
    }

    // Pump 500ms right after load at popup width (gridUnit * 19) to catch any polish oscillation.
    for (int turn = 0; turn < 60; ++turn) {
        view.requestUpdate();
        QCoreApplication::processEvents();
        QTest::qWait(8);
    }
    qInfo() << "initial 500ms: implicitWidthChanges=" << implicitWidthChanges
            << "frameWidthChanges=" << widthChanges
            << "descendantWidthChanges=" << allDescendantWidthChanges
            << "gridUnit=" << rootObj->property("themeObj").value<QObject *>()->property("gridUnit")
            << "scaleFactor=" << rootObj->property("themeObj").value<QObject *>()->property("scaleFactor")
            << "rootImplicitWidth=" << qobject_cast<QQuickItem *>(rootObj)->implicitWidth()
            << "frameWidth=" << frame->width();
    for (QQuickItem *child : qobject_cast<QQuickItem *>(rootObj)->findChildren<QQuickItem *>()) {
        if (child->implicitWidth() > 300) {
            qInfo() << "wide item:" << child->metaObject()->className() << child->objectName()
                    << "implicitWidth=" << child->implicitWidth() << "width=" << child->width();
        }
    }
    QVERIFY2(allDescendantWidthChanges == 0,
             qPrintable(QStringLiteral("Descendant widths oscillated (%1 changes) in initial 500ms")
                            .arg(allDescendantWidthChanges)));

    // Resize the popup across widths that wrap/unwrap Flow (including 0 width
    // during open/close) and toggle the channel selection (showAll=false -> true -> false).
    // CyberFrame.implicitWidth must never change (no feedback into the layout),
    // and frame.width must settle within 1 change per resize.
    const int widths[] = {0, 210, 260, 320, 360, 280};
    for (int w : widths) {
        implicitWidthChanges = 0;
        widthChanges = 0;
        rootObj->setProperty("width", w);
        for (int turn = 0; turn < 20; ++turn) {
            QCoreApplication::processEvents();
            QTest::qWait(5);
        }
        QVERIFY2(implicitWidthChanges == 0,
                 qPrintable(QStringLiteral("CyberFrame.implicitWidth changed %1 times at width %2")
                                .arg(implicitWidthChanges)
                                .arg(w)));
        QVERIFY2(widthChanges <= 2,
                 qPrintable(QStringLiteral("CyberFrame.width oscillated (%1 changes) at width %2")
                                .arg(widthChanges)
                                .arg(w)));
    }

    const int selections[] = {-1, 0, 1, -1, 0};
    for (int sel : selections) {
        implicitWidthChanges = 0;
        widthChanges = 0;
        cycle->setProperty("selected", sel);
        for (int turn = 0; turn < 20; ++turn) {
            QCoreApplication::processEvents();
            QTest::qWait(5);
        }
        QVERIFY2(implicitWidthChanges == 0,
                 qPrintable(QStringLiteral("CyberFrame.implicitWidth changed %1 times on selected=%2")
                                .arg(implicitWidthChanges)
                                .arg(sel)));
        QVERIFY2(widthChanges <= 1,
                 qPrintable(QStringLiteral("CyberFrame.width changed %1 times on selected=%2")
                                .arg(widthChanges)
                                .arg(sel)));
    }
}

// The Displays page inside the real popup structure must reach a fixed point:
// once settled, a second of frames may not change any item's size. A Row whose
// implicit width flips by one pixel per frame (fractional text advances against
// pixel-snapped layout allocations) makes plasmashell polish forever.
void TestCyberpunkLayout::cyberpunkPageIsStableAtFractionalScale()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.kde.plasma.components as PlasmaComponents3
        import org.kde.kirigami as Kirigami
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod
        import "skins/cyberpunk" as Cyber

        Item {
            id: root
            property alias displays: displaysLoader.item
            readonly property real edgeInset: Math.round(Kirigami.Units.smallSpacing * theme.scaleFactor)
            width: theme.gridUnit * 19
            height: 560
            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow"; overrides: ({ uiScale: 115 }) }
            Mod.ModuleSettings { id: settings }
            PlasmaComponents3.ScrollView {
                id: panel
                anchors.fill: parent
                PlasmaComponents3.ScrollBar.horizontal.visible: false
                ColumnLayout {
                    x: root.edgeInset
                    width: panel.width - 2 * root.edgeInset - theme.gridUnit
                    spacing: theme.sectionSpacing
                    Loader {
                        id: cycleLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/LightCycle.qml"), {
                            theme: theme, client: client, settings: settings })
                    }
                    Loader {
                        id: displaysLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Displays.qml"), {
                            theme: theme, client: client, settings: settings })
                    }
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestPage.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *displays = rootObj->property("displays").value<QObject *>();
    for (int i = 0; i < 150 && !(displays && displays->property("available").toBool()); ++i) {
        QTest::qWait(20);
        displays = rootObj->property("displays").value<QObject *>();
    }
    QVERIFY(displays && displays->property("available").toBool());

    const auto pump = [&view](int frames) {
        for (int i = 0; i < frames; ++i) {
            view.requestUpdate();
            QCoreApplication::processEvents();
            QTest::qWait(8);
        }
    };
    pump(60);

    int widthChanges = 0;
    QHash<QString, int> perItem;
    const auto items = qobject_cast<QQuickItem *>(rootObj)->findChildren<QQuickItem *>();
    for (QQuickItem *item : items) {
        connect(item, &QQuickItem::widthChanged, this, [&widthChanges, &perItem, item]() {
            ++widthChanges;
            perItem[QString::fromLatin1(item->metaObject()->className())] += 1;
        });
    }
    // A bracketed readout reports a size on the device-pixel grid, so a layout
    // that snaps its allocation to pixels agrees with the size hint.
    const qreal dpr = view.effectiveDevicePixelRatio();
    int brackets = 0;
    for (QQuickItem *item : items) {
        if (!item->property("bracketColor").isValid()) {
            continue;
        }
        ++brackets;
        const double device = item->implicitWidth() * dpr;
        qInfo() << "bracket" << item->implicitWidth() << item->implicitHeight() << "dpr" << dpr << "device" << device;
        QVERIFY2(qAbs(device - std::round(device)) < 0.001,
                 qPrintable(QStringLiteral("Bracket implicitWidth %1 is off the device pixel grid (dpr %2)")
                                .arg(item->implicitWidth()).arg(dpr)));
        const double deviceH = item->implicitHeight() * dpr;
        QVERIFY2(qAbs(deviceH - std::round(deviceH)) < 0.001,
                 qPrintable(QStringLiteral("Bracket implicitHeight %1 is off the device pixel grid").arg(item->implicitHeight())));
    }
    QVERIFY(brackets > 0);
    // So does every positioner a layout places directly.
    for (QQuickItem *item : items) {
        const QString cls = QString::fromLatin1(item->metaObject()->className());
        const QString parentCls = item->parentItem() ? QString::fromLatin1(item->parentItem()->metaObject()->className()) : QString();
        if (cls.contains(QLatin1String("Layout")) || (!cls.startsWith(QLatin1String("QQuickRow")) && !cls.startsWith(QLatin1String("QQuickFlow")))) {
            continue;
        }
        if (!parentCls.contains(QLatin1String("Layout"))) {
            continue;
        }
        if (cls.startsWith(QLatin1String("QQuickFlow"))) {
            // A wrapping container's width is the layout's to give.
            const QVariant preferred = QQmlProperty(item, QStringLiteral("Layout.preferredWidth"),
                                                    qmlContext(item)).read();
            QVERIFY2(preferred.toDouble() == 0.0,
                     "A Flow in a layout must not offer a fractional implicit width (Layout.preferredWidth: 0)");
            continue;
        }
        const double device = item->implicitWidth() * dpr;
        qInfo() << "positioner" << cls << "in" << parentCls << item->implicitWidth() << "device" << device;
        QVERIFY2(qAbs(device - std::round(device)) < 0.001,
                 qPrintable(QStringLiteral("%1 in %2: implicitWidth %3 is off the device pixel grid")
                                .arg(cls, parentCls).arg(item->implicitWidth())));
    }

    // Open every channel's limit sliders and give them real-world labels.
    int channel = 0;
    for (QQuickItem *item : items) {
        if (item->property("limitsOpen").isValid()) {
            item->setProperty("limitsOpen", true);
            item->setProperty("stateText", QStringLiteral("MAN"));
            if (channel++ == 0) {
                item->setProperty("name", QStringLiteral("Dell U2723QE Display Port Long"));
            }
        }
    }
    pump(30);
    // Popup sizes as Plasma hands them out: fractional widths, short heights
    // that make the page scroll, tall ones that do not.
    const double widths[] = {312.4, 340.0, 351.7, 380.3, 402.9};
    const double heights[] = {260.0, 420.0, 560.0, 900.0};
    for (double w : widths) {
        for (double h : heights) {
            rootObj->setProperty("width", w);
            rootObj->setProperty("height", h);
            pump(25);
            widthChanges = 0;
            perItem.clear();
            pump(60);
            qInfo() << "scale" << qgetenv("QT_SCALE_FACTOR") << "size" << w << h
                    << "width changes in ~0.5s:" << widthChanges << perItem;
            QVERIFY2(widthChanges == 0,
                     qPrintable(QStringLiteral("%1 item width changes after the page settled at %2x%3")
                                    .arg(widthChanges).arg(w).arg(h)));
        }
    }
}

void TestCyberpunkLayout::cyberpunkHoverScrubDoesNotOscillate()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.kde.plasma.components as PlasmaComponents3
        import org.kde.kirigami as Kirigami
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod

        Item {
            id: root
            property alias lightCycle: cycleLoader.item
            readonly property real edgeInset: Math.round(Kirigami.Units.smallSpacing * theme.scaleFactor)
            width: 352
            height: 664
            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow"; overrides: ({ uiScale: 120 }) }
            Mod.ModuleSettings { id: settings }
            PlasmaComponents3.ScrollView {
                id: panel
                anchors.fill: parent
                PlasmaComponents3.ScrollBar.horizontal.visible: false
                ColumnLayout {
                    x: root.edgeInset
                    width: panel.width - 2 * root.edgeInset - theme.gridUnit
                    spacing: theme.sectionSpacing
                    Loader {
                        id: cycleLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        clip: true
                        Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/LightCycle.qml"), {
                            theme: theme, client: client, settings: settings })
                    }
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestHover.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *cycle = rootObj->property("lightCycle").value<QObject *>();
    for (int i = 0; i < 150 && !(cycle && cycle->property("available").toBool()); ++i) {
        QTest::qWait(20);
        cycle = rootObj->property("lightCycle").value<QObject *>();
    }
    QVERIFY(cycle && cycle->property("available").toBool());

    const auto pump = [&view](int frames) {
        for (int i = 0; i < frames; ++i) {
            view.requestUpdate();
            QCoreApplication::processEvents();
            QTest::qWait(5);
        }
    };
    pump(30);

    auto *cycleItem = qobject_cast<QQuickItem *>(cycle);
    QVERIFY(cycleItem != nullptr);

    QQuickItem *hoverArea = nullptr;
    for (QQuickItem *item : cycleItem->findChildren<QQuickItem *>()) {
        if (item->property("hoverEnabled").toBool() && item->property("time").isValid()) {
            hoverArea = item;
            break;
        }
    }
    QVERIFY(hoverArea != nullptr);

    int widthChanges = 0;
    for (QQuickItem *item : cycleItem->findChildren<QQuickItem *>()) {
        connect(item, &QQuickItem::widthChanged, this, [&widthChanges]() {
            ++widthChanges;
        });
    }

    // Sweep mouse across the plot (including the sunrise transition and right edge)
    // and verify that once positioned at a single point, layout widths do not oscillate.
    const QPointF origin = hoverArea->mapToScene(QPointF(0, hoverArea->height() * 0.5));
    const double plotW = hoverArea->width();
    for (int step = 0; step <= 40; ++step) {
        const double x = (plotW + 16.0) * step / 40.0;
        const QPoint pt = (origin + QPointF(x, 0)).toPoint();
        QTest::mouseMove(&view, pt);
        pump(4);
        widthChanges = 0;
        pump(20);
        QVERIFY2(widthChanges == 0,
                 qPrintable(QStringLiteral("Hover scrub caused %1 layout width oscillations at x=%2")
                                .arg(widthChanges)
                                .arg(x)));
    }
}

void TestCyberpunkLayout::weatherMetersUseArrowCursor()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod

        Item {
            id: root
            property alias weatherItem: weatherLoader.item
            property alias displaysItem: displaysLoader.item
            width: 418
            height: 760
            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow" }
            Mod.ModuleSettings { id: settings }
            ColumnLayout {
                width: root.width
                spacing: theme.sectionSpacing
                Loader {
                    id: weatherLoader
                    Layout.fillWidth: true
                    Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Weather.qml"), {
                        theme: theme, client: client, settings: settings })
                }
                Loader {
                    id: displaysLoader
                    Layout.fillWidth: true
                    Component.onCompleted: setSource(Qt.resolvedUrl("skins/cyberpunk/Displays.qml"), {
                        theme: theme, client: client, settings: settings })
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestWeatherCursor.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *weather = rootObj->property("weatherItem").value<QObject *>();
    QObject *displays = rootObj->property("displaysItem").value<QObject *>();
    for (int i = 0; i < 150 && !(weather && weather->property("available").toBool()
                                   && displays && displays->property("available").toBool()); ++i) {
        QTest::qWait(20);
        weather = rootObj->property("weatherItem").value<QObject *>();
        displays = rootObj->property("displaysItem").value<QObject *>();
    }
    QVERIFY(weather && weather->property("available").toBool());
    QVERIFY(displays && displays->property("available").toBool());
    for (int i = 0; i < 15; ++i) {
        view.requestUpdate();
        QCoreApplication::processEvents();
        QTest::qWait(5);
    }

    auto *weatherItem = qobject_cast<QQuickItem *>(weather);
    auto *displaysItem = qobject_cast<QQuickItem *>(displays);
    QVERIFY(weatherItem != nullptr);
    QVERIFY(displaysItem != nullptr);

    const auto collectVisualItems = [](QQuickItem *rootItem) {
        QList<QQuickItem *> result;
        QList<QQuickItem *> stack{rootItem};
        while (!stack.isEmpty()) {
            QQuickItem *cur = stack.takeLast();
            result.append(cur);
            for (QQuickItem *ch : cur->childItems()) {
                stack.append(ch);
            }
        }
        return result;
    };

    int weatherMouseAreas = 0;
    for (QQuickItem *item : collectVisualItems(weatherItem)) {
        if (QString::fromLatin1(item->metaObject()->className()).startsWith(QLatin1String("QQuickMouseArea"))) {
            ++weatherMouseAreas;
            QVERIFY2(!item->property("enabled").toBool(), "ForecastStrip CellBar meters must not be enabled");
            QCOMPARE(item->cursor().shape(), Qt::ArrowCursor);
        }
    }
    QVERIFY2(weatherMouseAreas >= 5,
             qPrintable(QStringLiteral("Expected >= 5 ForecastStrip CellBar MouseAreas, found %1")
                            .arg(weatherMouseAreas)));

    // Verify WeatherText helpers (with I18nStub providing i18nd/i18ndc)
    QObject *wt = nullptr;
    for (QObject *child : weather->findChildren<QObject *>()) {
        if (QString::fromLatin1(child->metaObject()->className()).contains(QLatin1String("WeatherText"))) {
            wt = child;
            break;
        }
    }
    QVERIFY(wt != nullptr);
    QVariant shortCapVar;
    QMetaObject::invokeMethod(wt, "shortConditionLabel", Q_RETURN_ARG(QVariant, shortCapVar),
                              Q_ARG(QVariant, QStringLiteral("partly_cloudy")), Q_ARG(QVariant, true));
    QCOMPARE(shortCapVar.toString(), QStringLiteral("P.CLDY"));
    QVariant detailsVar;
    QVariantMap sampleDetails{{QStringLiteral("feelsLikeC"), 14.2},
                              {QStringLiteral("humidityPercent"), 62},
                              {QStringLiteral("windSpeedMps"), 3.45},
                              {QStringLiteral("airQualityIndex"), 42}};
    QMetaObject::invokeMethod(wt, "detailsList", Q_RETURN_ARG(QVariant, detailsVar),
                              Q_ARG(QVariant, sampleDetails), Q_ARG(QVariant, QStringLiteral("caps")));
    const QVariantList detailsOut = detailsVar.toList();
    QCOMPARE(detailsOut.size(), 4);
    QCOMPARE(detailsOut.at(0).toMap().value(QStringLiteral("k")).toString(), QStringLiteral("FEELS"));

    QQuickItem *sliderCellBar = nullptr;
    QQuickItem *sliderMouseArea = nullptr;
    for (QQuickItem *item : collectVisualItems(displaysItem)) {
        if (item->property("interactive").toBool() && item->property("cells").isValid()) {
            sliderCellBar = item;
            for (QQuickItem *sub : collectVisualItems(item)) {
                if (QString::fromLatin1(sub->metaObject()->className()).startsWith(QLatin1String("QQuickMouseArea"))) {
                    sliderMouseArea = sub;
                    break;
                }
            }
            if (sliderMouseArea) {
                break;
            }
        }
    }
    QVERIFY(sliderCellBar != nullptr && sliderMouseArea != nullptr);
    QVERIFY(sliderMouseArea->property("enabled").toBool());
    QCOMPARE(sliderMouseArea->cursor().shape(), Qt::PointingHandCursor);

    sliderCellBar->setProperty("interactive", false);
    QCoreApplication::processEvents();
    QVERIFY(!sliderMouseArea->property("enabled").toBool());
    QCOMPARE(sliderMouseArea->cursor().shape(), Qt::ArrowCursor);

    sliderCellBar->setProperty("interactive", true);
    QCoreApplication::processEvents();
    QCOMPARE(sliderMouseArea->cursor().shape(), Qt::PointingHandCursor);
    sliderCellBar->setProperty("enabled", false);
    QCoreApplication::processEvents();
    QCOMPARE(sliderMouseArea->cursor().shape(), Qt::ArrowCursor);
}

void TestCyberpunkLayout::allSkinsSettleAndScrubWithoutOscillation()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.kde.plasma.components as PlasmaComponents3
        import org.kde.kirigami as Kirigami
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod
        import "modules/Modules.js" as Modules

        Item {
            id: root
            property alias client: client
            property alias themeObj: theme
            property alias lightCycle: cycleLoader.item
            property alias displays: displaysLoader.item
            property alias weather: weatherLoader.item
            property alias forecast: forecastLoader.item
            readonly property real edgeInset: Math.round(Kirigami.Units.smallSpacing * theme.scaleFactor)
            width: 352
            height: 664
            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow"; overrides: ({ uiScale: 120 }) }
            Mod.ModuleSettings { id: settings }
            Binding {
                target: theme
                property: "gridContentWidth"
                value: panel.width - 2 * root.edgeInset - theme.gridUnit
            }
            PlasmaComponents3.ScrollView {
                id: panel
                anchors.fill: parent
                PlasmaComponents3.ScrollBar.horizontal.visible: false
                ColumnLayout {
                    x: root.edgeInset
                    width: panel.width - 2 * root.edgeInset - theme.gridUnit
                    spacing: theme.sectionSpacing
                    Loader {
                        id: cycleLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        clip: true
                        readonly property string src: Modules.source("curve", theme.themeId, theme.renderer)
                        onSrcChanged: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                        Component.onCompleted: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                    }
                    Loader {
                        id: displaysLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        clip: true
                        readonly property string src: Modules.source("displays", theme.themeId, theme.renderer)
                        onSrcChanged: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                        Component.onCompleted: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                    }
                    Loader {
                        id: weatherLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        clip: true
                        readonly property string src: Modules.source("weather", theme.themeId, theme.renderer)
                        onSrcChanged: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                        Component.onCompleted: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                    }
                    Loader {
                        id: forecastLoader
                        Layout.fillWidth: true
                        Layout.maximumWidth: parent ? parent.width : 0
                        clip: true
                        readonly property string src: Modules.source("forecast", theme.themeId, theme.renderer)
                        onSrcChanged: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                        Component.onCompleted: setSource(Qt.resolvedUrl(src), {
                            theme: theme, client: client, settings: settings })
                    }
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestAllSkins.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *client = rootObj->property("client").value<QObject *>();
    QVERIFY(client != nullptr);

    const auto pump = [&view](int frames) {
        for (int i = 0; i < frames; ++i) {
            view.requestUpdate();
            QCoreApplication::processEvents();
            QTest::qWait(4);
        }
    };

    struct SkinSpec {
        const char *id;
        const char *renderer;
        const char *font;
    };
    const SkinSpec skins[] = {
        {"cyberpunk", "vector", "sans"},
        {"braun", "vector", "sans"},
        {"synthwave84", "vector", "display"},
        {"nothing", "vector", "sans"},
        {"ps3", "vector", "sans"},
        {"wii", "vector", "sans"},
        {"macos", "vector", "sans"},
        {"winxp", "vector", "sans"},
        {"casiodigital", "vector", "mono"},
        {"nds", "vector", "sans"},
        {"xbox360", "vector", "sans"},
        {"handheld", "pixel", "bitmap8"},
        {"amiga", "pixel", "bitmap8"},
        {"commodore64", "pixel", "bitmap_vga"},
        {"dracula", "terminal", "mono"},
        {"hackergreen", "terminal", "mono"},
    };

    for (const SkinSpec &skin : skins) {
        QTest::mouseMove(&view, QPoint(1, 1));
        daemon.setThemeSpec(QString::fromLatin1(skin.id),
                            QString::fromLatin1(skin.renderer),
                            QString::fromLatin1(skin.font));
        QMetaObject::invokeMethod(client, "queryStatus");
        QObject *themeObj = rootObj->property("themeObj").value<QObject *>();
        QVERIFY(themeObj != nullptr);
        for (int i = 0; i < 50 && themeObj->property("themeId").toString() != QLatin1String(skin.id); ++i) {
            QTest::qWait(10);
        }
        QCOMPARE(themeObj->property("themeId").toString(), QLatin1String(skin.id));
        pump(20);

        QObject *cycle = rootObj->property("lightCycle").value<QObject *>();
        QVERIFY2(cycle != nullptr, skin.id);
        for (int i = 0; i < 50 && !cycle->property("available").toBool(); ++i) {
            QTest::qWait(10);
            cycle = rootObj->property("lightCycle").value<QObject *>();
        }
        QVERIFY2(cycle && cycle->property("available").toBool(), skin.id);
        QObject *weather = rootObj->property("weather").value<QObject *>();
        QVERIFY2(weather && weather->property("available").toBool(), skin.id);
        QObject *forecast = rootObj->property("forecast").value<QObject *>();
        QVERIFY2(forecast != nullptr, skin.id);
        const QVariant hostedVar = forecast->property("hosted");
        if (!hostedVar.isValid() || !hostedVar.toBool()) {
            QVERIFY2(forecast->property("available").toBool(), skin.id);
        }

        const auto collectVisualItems = [](QQuickItem *itemRoot) {
            QList<QQuickItem *> result;
            QList<QQuickItem *> stack{itemRoot};
            while (!stack.isEmpty()) {
                QQuickItem *cur = stack.takeLast();
                result.append(cur);
                for (QQuickItem *ch : cur->childItems()) {
                    stack.append(ch);
                }
            }
            return result;
        };

        for (QObject *modObj : {weather, forecast}) {
            if (auto *modItem = qobject_cast<QQuickItem *>(modObj)) {
                for (QQuickItem *item : collectVisualItems(modItem)) {
                    if (QString::fromLatin1(item->metaObject()->className()).startsWith(QLatin1String("QQuickMouseArea"))) {
                        if (!item->isEnabled() || !item->property("enabled").toBool()) {
                            QVERIFY2(item->cursor().shape() != Qt::PointingHandCursor,
                                     qPrintable(QStringLiteral("Disabled MouseArea in skin %1 has PointingHandCursor")
                                                    .arg(QLatin1String(skin.id))));
                        }
                    }
                }
            }
        }

        for (double w : {352.0, 418.0}) {
            QTest::mouseMove(&view, QPoint(1, 1));
            rootObj->setProperty("width", w);
            pump(15);

            int sizeChanges = 0;
            const auto allItems = qobject_cast<QQuickItem *>(rootObj)->findChildren<QQuickItem *>();
            std::vector<QMetaObject::Connection> conns;
            conns.reserve(allItems.size() * 2);
            for (QQuickItem *item : allItems) {
                conns.push_back(connect(item, &QQuickItem::widthChanged, this, [&sizeChanges]() {
                    ++sizeChanges;
                }));
                conns.push_back(connect(item, &QQuickItem::heightChanged, this, [&sizeChanges]() {
                    ++sizeChanges;
                }));
            }

            pump(15);
            QVERIFY2(sizeChanges == 0,
                     qPrintable(QStringLiteral("Skin %1 oscillated (%2 size changes) while idle at width %3")
                                    .arg(QLatin1String(skin.id))
                                    .arg(sizeChanges)
                                    .arg(w)));

            auto *cycleItem = qobject_cast<QQuickItem *>(cycle);
            QQuickItem *hoverArea = nullptr;
            for (QQuickItem *item : cycleItem->findChildren<QQuickItem *>()) {
                if (item->property("hoverEnabled").toBool() && item->width() > 40 && item->height() > 15) {
                    hoverArea = item;
                    break;
                }
            }
            if (hoverArea) {
                const QPointF origin = hoverArea->mapToScene(QPointF(0, hoverArea->height() * 0.5));
                const double plotW = hoverArea->width();
                for (int step = 0; step <= 10; ++step) {
                    const double x = (plotW + 12.0) * step / 10.0;
                    QTest::mouseMove(&view, (origin + QPointF(x, 0)).toPoint());
                    pump(3);
                    sizeChanges = 0;
                    pump(10);
                    QVERIFY2(sizeChanges == 0,
                             qPrintable(QStringLiteral("Skin %1 oscillated (%2 size changes) on hover at x=%3 width=%4")
                                            .arg(QLatin1String(skin.id))
                                            .arg(sizeChanges)
                                            .arg(x)
                                            .arg(w)));
                }
            }

            for (const auto &c : conns) {
                disconnect(c);
            }
        }
    }
}

void TestCyberpunkLayout::rangeDrawerPersistsAcrossMonitorsChanged()
{
    QGuiApplication::setFont(QFont(QStringLiteral("Inter"), 12));
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import QtQuick.Layouts
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod
        import "modules/Modules.js" as Modules

        Item {
            id: root
            property alias client: client
            property alias themeObj: theme
            property alias displays: displaysLoader.item
            width: 380
            height: 760
            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow"; overrides: ({ uiScale: 100 }) }
            Mod.ModuleSettings { id: settings }
            Binding { target: theme; property: "gridContentWidth"; value: 360 }
            ColumnLayout {
                x: 8
                y: 8
                width: 360
                Loader {
                    id: displaysLoader
                    Layout.fillWidth: true
                    readonly property string src: Modules.source("displays", theme.themeId, theme.renderer)
                    onSrcChanged: setSource(Qt.resolvedUrl(src), {
                        theme: theme, client: client, settings: settings })
                    Component.onCompleted: setSource(Qt.resolvedUrl(src), {
                        theme: theme, client: client, settings: settings })
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestRangePersist.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *client = rootObj->property("client").value<QObject *>();
    QObject *themeObj = rootObj->property("themeObj").value<QObject *>();
    QVERIFY(client && themeObj);

    const auto pump = [&view](int frames) {
        for (int i = 0; i < frames; ++i) {
            view.requestUpdate();
            QCoreApplication::processEvents();
            QTest::qWait(8);
        }
    };

    struct SkinSpec {
        const char *id;
        const char *renderer;
        const char *font;
    };
    const SkinSpec skins[] = {
        {"cyberpunk", "vector", "sans"},
        {"nothing", "vector", "sans"},
        {"casiodigital", "vector", "mono"},
        {"amiga", "pixel", "bitmap8"},
        {"wii", "vector", "sans"},
        {"winxp", "vector", "sans"},
        {"xbox360", "vector", "sans"},
    };

    for (const SkinSpec &skin : skins) {
        daemon.setM1Percent(55);
        daemon.setThemeSpec(QString::fromLatin1(skin.id),
                            QString::fromLatin1(skin.renderer),
                            QString::fromLatin1(skin.font));
        QMetaObject::invokeMethod(client, "queryStatus");
        QTRY_VERIFY_WITH_TIMEOUT(themeObj->property("themeId").toString() == QLatin1String(skin.id), 3000);
        QTRY_VERIFY_WITH_TIMEOUT(client->property("monitors").toList().size() == 2, 3000);
        QObject *displays = nullptr;
        QTRY_VERIFY_WITH_TIMEOUT((displays = rootObj->property("displays").value<QObject *>()) != nullptr
                                     && displays->property("available").toBool(),
                                 3000);
        pump(20);

        auto *displaysItem = qobject_cast<QQuickItem *>(displays);
        QVERIFY(displaysItem != nullptr);

        // Traverse the QtQuick visual tree (`childItems()`) so Repeater delegates
        // are included alongside statically declared items.
        const auto collectItems = [](QQuickItem *root) {
            QList<QQuickItem *> result;
            QList<QQuickItem *> stack{root};
            while (!stack.isEmpty()) {
                QQuickItem *cur = stack.takeFirst();
                const auto kids = cur->childItems();
                for (QQuickItem *kid : kids) {
                    result.append(kid);
                    stack.append(kid);
                }
            }
            return result;
        };

        // Locate the UI toggle button for monitor DP-3 (15-85%) and click it.
        QQuickItem *toggleTarget = nullptr;
        const auto findToggle = [&]() -> QQuickItem * {
            const auto children = collectItems(displaysItem);
            if (QLatin1String(skin.id) == QLatin1String("nothing")) {
                for (QQuickItem *it : children) {
                    if (!it->isVisible() || !it->property("active").isValid()) {
                        continue;
                    }
                    const QVariant txt = it->property("text");
                    if (txt.isValid() && txt.toString().startsWith(QLatin1String("RANGE 15"))) {
                        return it;
                    }
                }
            } else if (QLatin1String(skin.id) == QLatin1String("cyberpunk")) {
                for (QQuickItem *it : children) {
                    if (it->isVisible() && it->property("bracketColor").isValid()
                        && it->property("value").toString().contains(QLatin1String("15"))
                        && it->property("value").toString().contains(QLatin1String("85"))) {
                        return it->parentItem() ? it->parentItem() : it;
                    }
                }
            } else if (QLatin1String(skin.id) == QLatin1String("casiodigital")) {
                for (QQuickItem *it : children) {
                    if (it->isVisible() && it->property("text").toString() == QLatin1String("15-85")) {
                        return it->parentItem() ? it->parentItem() : it;
                    }
                }
            } else if (QLatin1String(skin.id) == QLatin1String("amiga")) {
                QQuickItem *best = nullptr;
                qreal topY = 1e9;
                for (QQuickItem *it : children) {
                    if (it->isVisible() && it->property("text").toString() == QLatin1String("Lim")) {
                        const qreal sy = it->mapToScene(QPointF(0, 0)).y();
                        if (sy < topY) {
                            topY = sy;
                            best = it;
                        }
                    }
                }
                return best;
            } else if (QLatin1String(skin.id) == QLatin1String("wii")
                       || QLatin1String(skin.id) == QLatin1String("xbox360")) {
                for (QQuickItem *it : children) {
                    if (it->isVisible() && it->property("hint").isValid()
                        && it->property("hint").toString().contains(QLatin1String("15"))
                        && it->property("hint").toString().contains(QLatin1String("85"))) {
                        return it;
                    }
                }
            } else if (QLatin1String(skin.id) == QLatin1String("winxp")) {
                for (QQuickItem *it : children) {
                    if (it->isVisible() && it->property("text").toString() == QLatin1String("DP-3")) {
                        return it;
                    }
                }
            }
            return nullptr;
        };

        for (int attempt = 0; attempt < 30 && !toggleTarget; ++attempt) {
            toggleTarget = findToggle();
            if (!toggleTarget) {
                pump(2);
            }
        }
        QVERIFY2(toggleTarget != nullptr, qPrintable(QStringLiteral("No range toggle found for %1").arg(QLatin1String(skin.id))));
        const QPoint clickPt = toggleTarget->mapToScene(QPointF(toggleTarget->width() * 0.5, toggleTarget->height() * 0.5)).toPoint();
        QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, clickPt);
        pump(10);

        QTRY_VERIFY2_WITH_TIMEOUT(
            displays->property("expandedLimits").toMap().value(QStringLiteral("DP-3")).toBool(),
            qPrintable(QStringLiteral("Clicking range toggle did not open DP-3 drawer on %1").arg(QLatin1String(skin.id))),
            1500);

        // Simulate daemon brightness update causing monitorsChanged() -> Repeater rebuild.
        QSignalSpy monitorsSpy(client, SIGNAL(monitorsChanged()));
        daemon.setM1Percent(33);
        QMetaObject::invokeMethod(client, "queryStatus");
        QVERIFY2(monitorsSpy.wait(3000), qPrintable(QStringLiteral("monitorsChanged not emitted on %1").arg(QLatin1String(skin.id))));
        pump(15);

        QVERIFY2(displays->property("expandedLimits").toMap().value(QStringLiteral("DP-3")).toBool(),
                 qPrintable(QStringLiteral("DP-3 expandedLimits dropped across monitorsChanged on %1").arg(QLatin1String(skin.id))));

        bool delegateStillOpen = false;
        const auto updatedChildren = collectItems(displaysItem);
        for (QQuickItem *it : updatedChildren) {
            if (!it->isVisible()) {
                continue;
            }
            if ((it->property("limitsOpen").isValid() && it->property("limitsOpen").toBool())
                || (it->property("expanded").isValid() && it->property("expanded").toBool())) {
                delegateStillOpen = true;
                break;
            }
        }
        QVERIFY2(delegateStillOpen,
                 qPrintable(QStringLiteral("DP-3 delegate drawer closed after monitorsChanged on %1").arg(QLatin1String(skin.id))));
    }
}

void TestCyberpunkLayout::zeroPercentLeftEdgeReachableOnPixelSliders()
{
    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import "engine" as Engine
        import "skins/cyberpunk" as Cyber
        import "skins/casiodigital" as Casio
        import "skins/pixel" as Pix

        Item {
            id: root
            property alias cellBar: cellBar
            property alias casioBar: casioBar
            property alias pixelSlider: pixelSlider
            width: 300
            height: 220

            Engine.Theme { id: cyberTheme }
            Engine.Theme { id: casioTheme }
            Engine.Theme { id: pixelTheme }
            Cyber.CyberStyle { id: cyberStyle; theme: cyberTheme }
            Casio.CasioStyle { id: casioStyle; theme: casioTheme }
            Pix.PixelStyle { id: pixelStyle; theme: pixelTheme }

            Column {
                x: 10
                y: 10
                width: 240
                spacing: 12
                Cyber.CellBar {
                    id: cellBar
                    width: 240
                    height: 36
                    style: cyberStyle
                    interactive: true
                    value: 50
                }
                Casio.CasioBar {
                    id: casioBar
                    width: 240
                    height: 36
                    style: casioStyle
                    interactive: true
                    value: 50
                }
                Pix.PixelCellSlider {
                    id: pixelSlider
                    width: 240
                    theme: pixelTheme
                    style: pixelStyle
                    interactive: true
                    value: 50
                }
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestSliderBounds.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    const char *names[] = {"cellBar", "casioBar", "pixelSlider"};
    for (const char *name : names) {
        auto *slider = rootObj->property(name).value<QQuickItem *>();
        QVERIFY(slider != nullptr);
        QSignalSpy committedSpy(slider, SIGNAL(committed(int)));

        // Left edge reaches 0% when from == 0.
        slider->setProperty("from", 0);
        slider->setProperty("to", 100);
        slider->setProperty("value", 50);
        const QPoint leftPt = slider->mapToScene(QPointF(1.0, slider->height() * 0.5)).toPoint();
        QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, leftPt);
        QTRY_VERIFY2_WITH_TIMEOUT(committedSpy.count() >= 1, name, 1000);
        QCOMPARE(committedSpy.last().at(0).toInt(), 0);

        // Constrained range [12, 75] clamps both edges to from..to.
        committedSpy.clear();
        slider->setProperty("from", 12);
        slider->setProperty("to", 75);
        slider->setProperty("value", 45);
        QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, leftPt);
        QTRY_VERIFY_WITH_TIMEOUT(committedSpy.count() >= 1, 1000);
        QCOMPARE(committedSpy.last().at(0).toInt(), 12);

        committedSpy.clear();
        const QPoint rightPt = slider->mapToScene(QPointF(slider->width() - 2.0, slider->height() * 0.5)).toPoint();
        QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, rightPt);
        QTRY_VERIFY_WITH_TIMEOUT(committedSpy.count() >= 1, 1000);
        QCOMPARE(committedSpy.last().at(0).toInt(), 75);
    }
}

void TestCyberpunkLayout::instrumentPausedArrowOpensMenuAndTracksActivePopupMenus()
{
    QTemporaryDir tmpDir;
    QVERIFY(tmpDir.isValid());
    const QString sockPath = tmpDir.filePath(QStringLiteral("control.sock"));
    MockDaemon daemon(sockPath);
    QVERIFY(daemon.isListening());

    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.rootContext()->setContextProperty(QStringLiteral("testSocketPath"), sockPath);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const QByteArray qml = R"(
        import QtQuick
        import org.sunreactor.plasma
        import "engine" as Engine
        import "modules" as Mod
        import "skins/instrument" as Inst

        Item {
            id: host
            property int activePopupMenus: 0
            property alias client: client
            property alias automation: automation
            width: 380
            height: 280

            SunReactorClient { id: client; socketPath: testSocketPath }
            Engine.Theme { id: theme; spec: client.theme; source: "follow"; overrides: ({ uiScale: 100 }) }
            Mod.ModuleSettings { id: settings }

            Inst.Automation {
                id: automation
                x: 10
                y: 10
                width: 340
                height: implicitHeight
                theme: theme
                client: client
                settings: settings
            }
        }
    )";
    QQmlComponent comp(view.engine());
    comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestInstArrow.qml")));
    QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
    QObject *rootObj = comp.create();
    QVERIFY(rootObj != nullptr);
    view.setContent(QUrl(), &comp, rootObj);
    view.show();
    QVERIFY(QTest::qWaitForWindowExposed(&view));

    QObject *client = rootObj->property("client").value<QObject *>();
    QTRY_VERIFY_WITH_TIMEOUT(client->property("isConnected").toBool(), 3000);
    auto *automation = rootObj->property("automation").value<QQuickItem *>();
    QVERIFY(automation != nullptr);

    QQuickItem *arrowHit = nullptr;
    QList<QQuickItem *> stack{automation};
    while (!stack.isEmpty() && !arrowHit) {
        QQuickItem *cur = stack.takeFirst();
        const auto kids = cur->childItems();
        for (QQuickItem *it : kids) {
            if (it->isVisible() && qFuzzyCompare(it->z(), 1.0) && it->width() > 0 && it->height() > 0) {
                arrowHit = it;
                break;
            }
            stack.append(it);
        }
    }
    QVERIFY2(arrowHit != nullptr && arrowHit->isVisible(), "Paused arrowHit item not found");

    const QPoint pt = arrowHit->mapToScene(QPointF(arrowHit->width() * 0.5, arrowHit->height() * 0.5)).toPoint();
    QTest::mouseClick(&view, Qt::LeftButton, Qt::NoModifier, pt);
    QTRY_COMPARE_WITH_TIMEOUT(rootObj->property("activePopupMenus").toInt(), 1, 2000);

    QObject *openedMenu = nullptr;
    for (QObject *obj : automation->findChildren<QObject *>()) {
        if (obj->property("_overlayRegistered").isValid() && obj->property("visible").toBool()) {
            openedMenu = obj;
            break;
        }
    }
    QVERIFY2(openedMenu != nullptr, "ModeDurationMenu should be open");

    QMetaObject::invokeMethod(openedMenu, "close");
    QTRY_COMPARE_WITH_TIMEOUT(rootObj->property("activePopupMenus").toInt(), 0, 2000);
}

void TestCyberpunkLayout::tabBarsStepWithLeftRightKeys()
{
    I18nStub i18nStub;
    QQuickView view;
    view.engine()->addImportPath(QStringLiteral(QML_IMPORT_DIR));
    view.rootContext()->setContextObject(&i18nStub);
    view.setResizeMode(QQuickView::SizeViewToRootObject);

    const char *tabUrls[] = {
        "skins/amiga/Tabs.qml",
        "skins/commodore64/Tabs.qml",
        "skins/nds/Tabs.qml",
        "components/tabs/PixelTabs.qml",
    };

    for (const char *relUrl : tabUrls) {
        const QByteArray qml = R"(
            import QtQuick
            import "engine" as Engine

            Item {
                id: host
                property alias tabBar: loader.item
                width: 360
                height: 80
                Engine.Theme { id: theme }
                function loadTab(url) {
                    loader.setSource(url, {
                        theme: theme,
                        tabs: [{ id: "brightness", text: "Brightness", hint: "" },
                               { id: "weather", text: "Weather", hint: "" }],
                        current: "brightness"
                    });
                }
                Loader {
                    id: loader
                    objectName: "tabLoader"
                    x: 8
                    y: 8
                    width: 344
                }
            }
        )";
        QQmlComponent comp(view.engine());
        comp.setData(qml, QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QStringLiteral("/TestTabKeys.qml")));
        QVERIFY2(comp.isReady(), qPrintable(comp.errorString()));
        QObject *rootObj = comp.create();
        QVERIFY(rootObj != nullptr);
        view.setContent(QUrl(), &comp, rootObj);
        view.show();
        QVERIFY(QTest::qWaitForWindowExposed(&view));

        const QUrl targetUrl = QUrl::fromLocalFile(QStringLiteral(PLASMA_UI_DIR) + QLatin1Char('/') + QLatin1String(relUrl));
        QMetaObject::invokeMethod(rootObj, "loadTab", Q_ARG(QVariant, targetUrl));
        QQuickItem *tabBar = nullptr;
        QTRY_VERIFY_WITH_TIMEOUT((tabBar = rootObj->property("tabBar").value<QQuickItem *>()) != nullptr, 1500);

        QVERIFY2(tabBar->activeFocusOnTab(), relUrl);
        QSignalSpy pickedSpy(tabBar, SIGNAL(picked(QString)));
        tabBar->forceActiveFocus();
        QTRY_VERIFY_WITH_TIMEOUT(tabBar->hasActiveFocus(), 1000);

        QTest::keyClick(&view, Qt::Key_Right);
        QTRY_COMPARE_WITH_TIMEOUT(pickedSpy.count(), 1, 1000);
        QCOMPARE(pickedSpy.at(0).at(0).toString(), QStringLiteral("weather"));

        tabBar->setProperty("current", QStringLiteral("weather"));
        QTest::keyClick(&view, Qt::Key_Left);
        QTRY_COMPARE_WITH_TIMEOUT(pickedSpy.count(), 2, 1000);
        QCOMPARE(pickedSpy.at(1).at(0).toString(), QStringLiteral("brightness"));
    }
}

QTEST_MAIN(TestCyberpunkLayout)
#include "test_cyberpunk_layout.moc"
