#include <QElapsedTimer>
#include <QImage>
#include <QTest>

#include "pixelart.h"

class TestPixelArt : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void longElisionIsLinearEnough();
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

QTEST_MAIN(TestPixelArt)
#include "test_pixelart.moc"
