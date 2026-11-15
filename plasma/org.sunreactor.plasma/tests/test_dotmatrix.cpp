// Dot-matrix text: glyph coverage, pitch snapping and the cell map. No window
// is created; the scene graph is only exercised by the headless gallery.

#include "dotmatrix.h"

#include <QTest>
#include <QVariantMap>

#include <climits>
#include <limits>

class TestDotMatrix : public QObject
{
    Q_OBJECT

private Q_SLOTS:
    void smallFaceCoversTheReadout();
    void tinyFaceUppercasesAndCoversDigits();
    void pitchSnapsToWholeDevicePixels();
    void capitalIMapsToItsGlyph();
    void descendersAddRows();
    void accentIndexMarksOneCharacter();
    void hostileInputsStayBounded();
};

void TestDotMatrix::smallFaceCoversTheReadout()
{
    DotText text;
    QVERIFY(text.covers(QStringLiteral("0123456789")));
    QVERIFY(text.covers(QStringLiteral("ABCDEFGHIJKLMNOPQRSTUVWXYZ")));
    QVERIFY(text.covers(QStringLiteral("abcdefghijklmnopqrstuvwxyz")));
    QVERIFY(text.covers(QStringLiteral(".,:;!?-+/()%'\"<>=_*&@|~^$#[]")));
    QVERIFY(text.covers(QString::fromUtf8("NEXT 16:20 -> 23% · 15°")));
    QVERIFY(text.covers(QString::fromUtf8("ÇĞİÖŞÜçğıöşüÄäÖöÜüßéÉ")));
    QVERIFY(!text.covers(QString::fromUtf8("€")));
}

void TestDotMatrix::tinyFaceUppercasesAndCoversDigits()
{
    DotText text;
    text.setFace(QStringLiteral("tiny"));
    QVERIFY(text.covers(QStringLiteral("0123456789 ABC:.%-")));
    QVERIFY(text.covers(QStringLiteral("abc")));
    QVERIFY(text.covers(QString::fromUtf8("şğ")));
    QVERIFY(!text.covers(QStringLiteral("@")));
    QCOMPARE(text.rowsPerGlyph(), 5);
}

void TestDotMatrix::pitchSnapsToWholeDevicePixels()
{
    QCOMPARE(DotField::devicePitch(6.0, 1.0), 6);
    QCOMPARE(DotField::devicePitch(6.0, 1.35), 8);
    QCOMPARE(DotField::devicePitch(6.0, 2.0), 12);
    QCOMPARE(DotField::devicePitch(4.0, 1.35), 5);
    QCOMPARE(DotField::devicePitch(1.0, 1.0), 3);
}

void TestDotMatrix::capitalIMapsToItsGlyph()
{
    DotText text;
    text.setText(QStringLiteral("I"));
    // 'I' is "###|.#.|.#.|.#.|.#.|.#.|###": three columns, seven rows.
    QCOMPARE(text.cols(), 3);
    QCOMPARE(text.rows(), 7);
    QCOMPARE(text.cell(0, 0), static_cast<int>(DotField::Lit));
    QCOMPARE(text.cell(1, 0), static_cast<int>(DotField::Lit));
    QCOMPARE(text.cell(0, 1), static_cast<int>(DotField::Off));
    QCOMPARE(text.cell(1, 1), static_cast<int>(DotField::Lit));
    QCOMPARE(text.cell(0, 6), static_cast<int>(DotField::Lit));

    text.setSize(2);
    QCOMPARE(text.cols(), 6);
    QCOMPARE(text.rows(), 14);
    QCOMPARE(text.cell(5, 13), static_cast<int>(DotField::Lit));
}

void TestDotMatrix::descendersAddRows()
{
    DotText text;
    text.setText(QStringLiteral("g"));
    QCOMPARE(text.descentRows(), 2);
    QCOMPARE(text.rows(), 9);

    text.setText(QStringLiteral("A"));
    QCOMPARE(text.descentRows(), 0);
    QCOMPARE(text.rows(), 7);
}

void TestDotMatrix::accentIndexMarksOneCharacter()
{
    DotText text;
    text.setText(QStringLiteral("AB"));
    text.setAccentIndex(1);
    // 'A' occupies columns 0..4, one spacing column, then 'B' from column 6.
    QCOMPARE(text.cell(1, 0), static_cast<int>(DotField::Lit));
    QCOMPARE(text.cell(6, 0), static_cast<int>(DotField::Accent));
    QCOMPARE(text.cell(7, 0), static_cast<int>(DotField::Accent));
    QCOMPARE(text.cell(5, 0), static_cast<int>(DotField::Blank));
}

void TestDotMatrix::hostileInputsStayBounded()
{
    DotField field;
    field.setCols(INT_MAX);
    field.setRows(INT_MAX);
    QVERIFY(field.cols() <= 512);
    QVERIFY(field.rows() <= 512);
    QVERIFY(static_cast<qint64>(field.cols()) * field.rows() <= 16 * 1024);
    QCOMPARE(DotField::devicePitch(std::numeric_limits<qreal>::quiet_NaN(), 1.0), 3);
    QCOMPARE(DotField::devicePitch(std::numeric_limits<qreal>::infinity(), 1.0), 3);

    DotText text;
    text.setText(QString(100000, QLatin1Char('A')));
    QVERIFY(text.cols() <= 512);
    QVERIFY(text.rows() <= 512);
    QVERIFY(text.covers(QString(100000, QLatin1Char('A'))) == false);

    DotCurve curve;
    QVariantList samples;
    for (int i = 0; i < 10000; ++i) {
        samples.append(QVariantMap{{QStringLiteral("t"), static_cast<double>(i + 1)},
                                   {QStringLiteral("pct"), static_cast<double>(i % 101)}});
    }
    samples.append(QVariantMap{{QStringLiteral("t"), std::numeric_limits<qreal>::quiet_NaN()},
                               {QStringLiteral("pct"), 50.0}});
    curve.setSamples(samples);
    curve.setT0(std::numeric_limits<qreal>::infinity());
    curve.setT1(std::numeric_limits<qreal>::quiet_NaN());
    curve.setNowT(std::numeric_limits<qreal>::quiet_NaN());
    curve.setMaxPct(std::numeric_limits<qreal>::infinity());
    QVERIFY(curve.cols() <= 512);
    QVERIFY(curve.rows() <= 512);

    DotRing ring;
    ring.setDiameter(INT_MAX);
    ring.setDots(INT_MAX);
    ring.setValue(std::numeric_limits<qreal>::quiet_NaN());
    QVERIFY(ring.cols() <= 64);
    QVERIFY(ring.rows() <= 64);

    DotBar bar;
    bar.setValue(std::numeric_limits<qreal>::infinity());
    QVERIFY(bar.cols() <= 512);

    DotIcon icon;
    icon.setCondition(QString(100000, QLatin1Char('x')));
    QVERIFY(icon.cols() == 12);
    QVERIFY(icon.rows() == 12);

    quint32 state = 0x243f6a88u;
    const auto next = [&state]() {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        return state;
    };
    for (int i = 0; i < 256; ++i) {
        field.setCols(static_cast<int>(next()));
        field.setRows(static_cast<int>(next()));
        field.setPitch(static_cast<qreal>(next()) / 1000.0);
        field.setDotScale(static_cast<qreal>(next()) / 1000.0);
        QVERIFY(static_cast<qint64>(field.cols()) * field.rows() <= 16 * 1024);
    }
}

QTEST_GUILESS_MAIN(TestDotMatrix)
#include "test_dotmatrix.moc"
