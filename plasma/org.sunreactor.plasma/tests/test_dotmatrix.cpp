// Dot-matrix text: glyph coverage, pitch snapping and the cell map. No window
// is created; the scene graph is only exercised by the headless gallery.

#include "dotmatrix.h"

#include <QTest>

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

QTEST_GUILESS_MAIN(TestDotMatrix)
#include "test_dotmatrix.moc"
