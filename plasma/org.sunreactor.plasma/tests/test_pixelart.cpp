#include <QElapsedTimer>
#include <QColor>
#include <QImage>
#include <QTest>
#include <QVariantMap>

#include <climits>
#include <limits>

#include "pixelart.h"

class TestPixelArt : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void longElisionIsLinearEnough();
    void hostileInputsStayBounded();
};

class TestGlyphs : public PixelGlyphs
{
public:
    using PixelGlyphs::PixelGlyphs;

    QImage renderForTest() const
    {
        return render();
    }
};

class TestCurve : public PixelCurve
{
public:
    using PixelCurve::PixelCurve;

    QImage renderForTest() const
    {
        return render();
    }
};

class TestSprite : public PixelSprite
{
public:
    using PixelSprite::PixelSprite;

    QImage renderForTest() const
    {
        return render();
    }
};

void TestPixelArt::longElisionIsLinearEnough()
{
    TestGlyphs glyphs;
    glyphs.setMaximumArtWidth(64);
    glyphs.setText(QString(60000, QLatin1Char('A')));

    QElapsedTimer timer;
    timer.start();
    const QImage image = glyphs.renderForTest();

    QVERIFY(!image.isNull());
    QVERIFY(image.width() <= 64);
    QVERIFY2(timer.elapsed() < 50, qPrintable(QStringLiteral("layout took %1 ms").arg(timer.elapsed())));
}

void TestPixelArt::hostileInputsStayBounded()
{
    TestGlyphs glyphs;
    glyphs.setText(QString(100000, QLatin1Char('A')));
    glyphs.setMaximumArtWidth(INT_MAX);
    glyphs.setPitch(INT_MAX);
    glyphs.setSegmentLength(INT_MAX);
    glyphs.setSegmentWidth(INT_MAX);
    const QImage glyphImage = glyphs.renderForTest();
    QVERIFY(glyphImage.width() <= 4096);
    QVERIFY(glyphImage.height() <= 4096);

    TestCurve curve;
    curve.setWidth(1000000000.0);
    curve.setHeight(3.0);
    curve.setArtScale(std::numeric_limits<qreal>::quiet_NaN());
    curve.setSamples({QVariantMap{{QStringLiteral("t"), 1.0}, {QStringLiteral("pct"), 50.0}},
                      QVariantMap{{QStringLiteral("t"), std::numeric_limits<qreal>::infinity()},
                                  {QStringLiteral("pct"), 20.0}}});
    const QImage curveImage = curve.renderForTest();
    QVERIFY(curveImage.width() <= 4096);
    QVERIFY(curveImage.height() <= 4096);
    curve.setWidth(3.0);
    curve.setHeight(1000000000.0);
    const QImage curveImageTall = curve.renderForTest();
    QVERIFY(curveImageTall.width() <= 4096);
    QVERIFY(curveImageTall.height() <= 4096);

    TestSprite sprite;
    sprite.setFrame(INT_MAX);
    sprite.setFrameWidth(INT_MAX);
    sprite.setFrameHeight(INT_MAX);
    sprite.setColors(QVariantList(10000, QColor(Qt::white)));
    QVERIFY(sprite.renderForTest().isNull());

    quint32 state = 0x9e3779b9u;
    const auto next = [&state]() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        return state;
    };
    for (int i = 0; i < 32; ++i) {
        glyphs.setMaximumArtWidth(static_cast<int>(next()));
        glyphs.setMaximumLines(static_cast<int>(next()));
        glyphs.setText(QString(static_cast<int>(next() % 512), QLatin1Char('Z')));
        const QImage image = glyphs.renderForTest();
        QVERIFY(image.width() <= 4096);
        QVERIFY(image.height() <= 4096);

        QVariantList samples;
        for (int j = 0; j < 16; ++j) {
            samples.append(QVariantMap{{QStringLiteral("t"), static_cast<double>(j + 1)},
                                       {QStringLiteral("pct"), static_cast<double>(next() % 101)}});
        }
        curve.setSamples(samples);
        curve.setWidth(static_cast<qreal>(next() % 1024));
        curve.setHeight(static_cast<qreal>(next() % 1024));
        const QImage image2 = curve.renderForTest();
        QVERIFY(image2.width() <= 4096);
        QVERIFY(image2.height() <= 4096);
    }
}

QTEST_MAIN(TestPixelArt)
#include "test_pixelart.moc"
