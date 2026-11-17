#include <QCoreApplication>
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

private:
    static QJsonObject statusPayload()
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
        widget[QStringLiteral("renderer")] = QStringLiteral("vector");
        widget[QStringLiteral("font")] = QStringLiteral("sans");
        widget[QStringLiteral("density")] = QStringLiteral("comfortable");
        widget[QStringLiteral("border")] = QStringLiteral("line");
        widget[QStringLiteral("radius")] = 0;
        widget[QStringLiteral("glow")] = 0.5;
        widget[QStringLiteral("modules")] = QJsonArray{
            QStringLiteral("telemetry"), QStringLiteral("curve"), QStringLiteral("displays"),
            QStringLiteral("automation"), QStringLiteral("weather"), QStringLiteral("forecast"),
            QStringLiteral("suntimes")};

        QJsonObject theme;
        theme[QStringLiteral("id")] = QStringLiteral("cyberpunk");
        theme[QStringLiteral("name")] = QStringLiteral("Cyberpunk");
        theme[QStringLiteral("family")] = QStringLiteral("instrument");
        theme[QStringLiteral("palette")] = palette;
        theme[QStringLiteral("widget")] = widget;

        QJsonObject m1;
        m1[QStringLiteral("logical_id")] = QStringLiteral("DP-3");
        m1[QStringLiteral("enabled")] = true;
        m1[QStringLiteral("last_applied_percent")] = 55;
        m1[QStringLiteral("min_pct")] = 15;
        m1[QStringLiteral("max_pct")] = 85;

        QJsonObject m2;
        m2[QStringLiteral("logical_id")] = QStringLiteral("DP-1");
        m2[QStringLiteral("enabled")] = true;
        m2[QStringLiteral("last_applied_percent")] = 60;
        m2[QStringLiteral("min_pct")] = 20;
        m2[QStringLiteral("max_pct")] = 90;

        QJsonObject status;
        status[QStringLiteral("now_epoch_s")] = 1700046000;
        status[QStringLiteral("sunrise_epoch_s")] = 1700024400;
        status[QStringLiteral("sunset_epoch_s")] = 1700060400;
        status[QStringLiteral("theme")] = theme;
        status[QStringLiteral("monitors")] = QJsonArray{m1, m2};
        return status;
    }

    static QJsonObject dayCurvePayload()
    {
        QJsonArray samples;
        const qint64 t0 = 1700002800;
        for (int i = 0; i < 96; ++i) {
            QJsonObject s;
            s[QStringLiteral("t")] = t0 + i * 900;
            const int pct = (i >= 24 && i <= 64) ? 65 : 20;
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
};

class I18nStub : public QObject
{
    Q_OBJECT
public:
    using QObject::QObject;
    Q_INVOKABLE QString i18nd(const QString &, const QString &text) const { return text; }
    Q_INVOKABLE QString i18ndc(const QString &, const QString &, const QString &text,
                               const QVariant &a1 = {}, const QVariant &a2 = {}) const
    {
        QString out = text;
        if (a1.isValid()) {
            out.replace(QStringLiteral("%1"), a1.toString());
        }
        if (a2.isValid()) {
            out.replace(QStringLiteral("%2"), a2.toString());
        }
        return out;
    }
};

class TestCyberpunkLayout : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void cyberpunkLightCycleSettlesOnResize();
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
    QVERIFY2(allDescendantWidthChanges <= 50,
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

QTEST_MAIN(TestCyberpunkLayout)
#include "test_cyberpunk_layout.moc"
