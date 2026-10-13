#pragma once

#include <QColor>
#include <QQuickItem>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <vector>

/*
 * Dot-matrix primitives for the Nothing skin (skins/nothing/BRAND.md).
 *
 * Every type is a grid of round dots drawn as ONE QSGGeometryNode per field:
 * each dot is a fan of triangles whose outer ring has alpha 0, so the vertex
 * colours feather the edge over one device pixel. Nothing is rasterised on the
 * CPU and nothing runs per frame: the geometry is rebuilt only when a property
 * changes. Centres sit on whole device pixels and the dot diameter is a whole
 * number of device pixels, so the grid is crisp at any device pixel ratio.
 *
 * Cells carry a state: DotField.Off (dim, unlit), Lit, Accent (red), Ghost
 * (outline) or Blank (not drawn). Off cells are drawn with dimColor, so an
 * unlit field looks like an LED matrix.
 */

/*!
 * A cols x rows dot grid. `pitch` is the requested logical pitch; the value it
 * reports is the one that makes `pitch * devicePixelRatio` a whole number of
 * device pixels (at least 3). The dot diameter is `dotScale` of the pitch,
 * rounded to whole device pixels.
 *
 * The derived types below fill the grid themselves and expose their own
 * inputs; their cols/rows are computed, so do not bind them there.
 */
class DotField : public QQuickItem
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(int cols READ cols WRITE setCols NOTIFY gridChanged)
    Q_PROPERTY(int rows READ rows WRITE setRows NOTIFY gridChanged)
    Q_PROPERTY(qreal pitch READ pitch WRITE setPitch NOTIFY pitchChanged)
    Q_PROPERTY(qreal dotScale READ dotScale WRITE setDotScale NOTIFY styleChanged)
    Q_PROPERTY(QColor litColor READ litColor WRITE setLitColor NOTIFY styleChanged)
    Q_PROPERTY(QColor accentColor READ accentColor WRITE setAccentColor NOTIFY styleChanged)
    Q_PROPERTY(QColor dimColor READ dimColor WRITE setDimColor NOTIFY styleChanged)
    Q_PROPERTY(QColor ghostColor READ ghostColor WRITE setGhostColor NOTIFY styleChanged)

public:
    enum Cell { Off = 0, Lit = 1, Accent = 2, Ghost = 3, Blank = 4 };
    Q_ENUM(Cell)

    explicit DotField(QQuickItem *parent = nullptr);

    /*! Pitch in device pixels for a requested logical pitch (>= 3 device px). */
    static int devicePitch(qreal pitch, qreal dpr);

    [[nodiscard]] int cols() const { return m_cols; }
    [[nodiscard]] int rows() const { return m_rows; }
    [[nodiscard]] qreal pitch() const;
    [[nodiscard]] qreal dotScale() const { return m_dotScale; }
    [[nodiscard]] QColor litColor() const { return m_lit; }
    [[nodiscard]] QColor accentColor() const { return m_accent; }
    [[nodiscard]] QColor dimColor() const { return m_dim; }
    [[nodiscard]] QColor ghostColor() const { return m_ghost; }

    void setCols(int v);
    void setRows(int v);
    void setPitch(qreal v);
    void setDotScale(qreal v);
    void setLitColor(const QColor &v);
    void setAccentColor(const QColor &v);
    void setDimColor(const QColor &v);
    void setGhostColor(const QColor &v);

    /*! Sets one cell; out-of-range coordinates are ignored. */
    Q_INVOKABLE void setCell(int col, int row, int cell);
    /*! The state of one cell (Blank outside the grid). */
    Q_INVOKABLE int cell(int col, int row) const;
    /*! Sets every cell to one state. */
    Q_INVOKABLE void fill(int cell);

Q_SIGNALS:
    void gridChanged();
    void pitchChanged();
    void styleChanged();

protected:
    /*! Recomputes the cells from the derived type's inputs. */
    virtual void refill() {}
    /*! refill(), then the implicit size and the geometry. */
    void refresh();
    /*! Resizes the grid (without any signal) and fills it with `fill`. */
    void setGrid(int cols, int rows, int fill = Off);
    /*! Writes one cell without touching the geometry (for refill()). */
    void put(int col, int row, int cell);

    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *data) override;
    void itemChange(ItemChange change, const ItemChangeData &value) override;
    void componentComplete() override;

private:
    void updateDpr();
    void updateImplicitSize();
    void markGeometry();

    int m_cols = 0;
    int m_rows = 0;
    std::vector<quint8> m_cells;
    qreal m_pitch = 6.0;
    qreal m_dotScale = 0.62;
    QColor m_lit = QColor(0xff, 0xff, 0xff);
    QColor m_accent = QColor(0xd7, 0x19, 0x21);
    QColor m_dim = QColor(255, 255, 255, 31);
    QColor m_ghost = QColor(255, 255, 255, 115);
    qreal m_dpr = 1.0;
    bool m_geometryDirty = true;
};

/*!
 * Text from the 5x7 (or 3x5) glyph tables, one dot per glyph pixel. `size` is
 * an integer scale: every glyph pixel becomes size x size dots. `letterSpacing`
 * is in glyph pixels (scaled with the glyph). `accentIndex` names one character
 * (code point index) to draw in accent. Characters the face lacks are a blank
 * cell; `covers()` says whether the face can show a string.
 *
 * Height: `rowsPerGlyph` is the capital box in dot rows; `descentRows` is the
 * extra rows the text needs for descenders (0 when none).
 */
class DotText : public DotField
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString text READ text WRITE setText NOTIFY textChanged)
    Q_PROPERTY(int size READ size WRITE setSize NOTIFY textChanged)
    /*! "small" (5x7 with lower case) or "tiny" (3x5 capitals). */
    Q_PROPERTY(QString face READ face WRITE setFace NOTIFY textChanged)
    Q_PROPERTY(int letterSpacing READ letterSpacing WRITE setLetterSpacing NOTIFY textChanged)
    Q_PROPERTY(int accentIndex READ accentIndex WRITE setAccentIndex NOTIFY textChanged)
    Q_PROPERTY(int rowsPerGlyph READ rowsPerGlyph NOTIFY textChanged)
    Q_PROPERTY(int descentRows READ descentRows NOTIFY textChanged)

public:
    explicit DotText(QQuickItem *parent = nullptr);

    [[nodiscard]] QString text() const { return m_text; }
    [[nodiscard]] int size() const { return m_size; }
    [[nodiscard]] QString face() const { return m_face; }
    [[nodiscard]] int letterSpacing() const { return m_spacing; }
    [[nodiscard]] int accentIndex() const { return m_accent; }
    [[nodiscard]] int rowsPerGlyph() const;
    [[nodiscard]] int descentRows() const;

    void setText(const QString &v);
    void setSize(int v);
    void setFace(const QString &v);
    void setLetterSpacing(int v);
    void setAccentIndex(int v);

    /*! True when every character of `text` exists in the current face. */
    Q_INVOKABLE bool covers(const QString &text) const;

protected:
    void refill() override;

private:
    [[nodiscard]] bool tiny() const;

Q_SIGNALS:
    void textChanged();

private:
    QString m_text;
    int m_size = 1;
    QString m_face = QStringLiteral("small");
    int m_spacing = 1;
    int m_accent = -1;
};

/*!
 * Horizontal gauge: `cols` dots (default 24) and `rows` (1 or 2). Lit up to
 * `value` (0..1); with `thumbAccent` the dot at the head is red and the head
 * stays visible at 0.
 */
class DotBar : public DotField
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(qreal value READ value WRITE setValue NOTIFY barChanged)
    Q_PROPERTY(bool thumbAccent READ thumbAccent WRITE setThumbAccent NOTIFY barChanged)

public:
    explicit DotBar(QQuickItem *parent = nullptr);

    [[nodiscard]] qreal value() const { return m_value; }
    [[nodiscard]] bool thumbAccent() const { return m_thumbAccent; }
    void setValue(qreal v);
    void setThumbAccent(bool v);

Q_SIGNALS:
    void barChanged();

protected:
    void refill() override;

private:
    qreal m_value = 0.0;
    bool m_thumbAccent = false;
};

/*!
 * Dotted circle: `dots` points around a `diameter` x `diameter` grid, centre
 * left blank. Lit clockwise from the top up to `value` (0..1); the head is red
 * unless `headAccent` is false.
 */
class DotRing : public DotField
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(qreal value READ value WRITE setValue NOTIFY ringChanged)
    Q_PROPERTY(int dots READ dots WRITE setDots NOTIFY ringChanged)
    Q_PROPERTY(int diameter READ diameter WRITE setDiameter NOTIFY ringChanged)
    Q_PROPERTY(bool headAccent READ headAccent WRITE setHeadAccent NOTIFY ringChanged)

public:
    explicit DotRing(QQuickItem *parent = nullptr);

    [[nodiscard]] qreal value() const { return m_value; }
    [[nodiscard]] int dots() const { return m_dots; }
    [[nodiscard]] int diameter() const { return m_diameter; }
    [[nodiscard]] bool headAccent() const { return m_headAccent; }
    void setValue(qreal v);
    void setDots(int v);
    void setDiameter(int v);
    void setHeadAccent(bool v);

Q_SIGNALS:
    void ringChanged();

protected:
    void refill() override;

private:
    qreal m_value = 0.0;
    int m_dots = 36;
    int m_diameter = 17;
    bool m_headAccent = true;
};

/*!
 * Day curve as a column-of-dots plot: x is time from t0 to t1, each of the
 * `cols` columns lit from the bottom up to the curve (0..maxPct). The column at
 * `nowT` is red. `samplesNoWeather`, when given, puts a dim outline dot at the
 * no-weather height of each column above the lit area (the weather cost).
 * Samples are lists of { t, pct }, sorted by t.
 */
class DotCurve : public DotField
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QVariantList samples READ samples WRITE setSamples NOTIFY curveChanged)
    Q_PROPERTY(QVariantList samplesNoWeather READ samplesNoWeather WRITE setSamplesNoWeather NOTIFY curveChanged)
    Q_PROPERTY(qreal t0 READ t0 WRITE setT0 NOTIFY curveChanged)
    Q_PROPERTY(qreal t1 READ t1 WRITE setT1 NOTIFY curveChanged)
    Q_PROPERTY(qreal nowT READ nowT WRITE setNowT NOTIFY curveChanged)
    Q_PROPERTY(qreal maxPct READ maxPct WRITE setMaxPct NOTIFY curveChanged)

public:
    explicit DotCurve(QQuickItem *parent = nullptr);

    struct Sample {
        qreal t;
        qreal pct;
    };

    [[nodiscard]] QVariantList samples() const { return m_samplesVar; }
    [[nodiscard]] QVariantList samplesNoWeather() const { return m_noWeatherVar; }
    [[nodiscard]] qreal t0() const { return m_t0; }
    [[nodiscard]] qreal t1() const { return m_t1; }
    [[nodiscard]] qreal nowT() const { return m_nowT; }
    [[nodiscard]] qreal maxPct() const { return m_maxPct; }
    void setSamples(const QVariantList &v);
    void setSamplesNoWeather(const QVariantList &v);
    void setT0(qreal v);
    void setT1(qreal v);
    void setNowT(qreal v);
    void setMaxPct(qreal v);

protected:
    void refill() override;

private:
    static std::vector<Sample> parse(const QVariantList &list);
    static qreal interpolate(const std::vector<Sample> &s, qreal t);

Q_SIGNALS:
    void curveChanged();

private:
    QVariantList m_samplesVar;
    std::vector<Sample> m_samples;
    QVariantList m_noWeatherVar;
    std::vector<Sample> m_noWeather;
    qreal m_t0 = 0;
    qreal m_t1 = 0;
    qreal m_nowT = -1;
    qreal m_maxPct = 100;
};

/*!
 * Weather pictogram on a fixed 12 x 12 grid. `condition` takes the WeatherGlyph
 * tokens (clear, partly_cloudy, cloudy, drizzle, rain, heavy_rain, thunderstorm,
 * snow, mist, fog, atmospheric; anything else is unknown). `night` swaps the
 * sun for a moon on clear and partly cloudy. Only the bolt is red.
 */
class DotIcon : public DotField
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString condition READ condition WRITE setCondition NOTIFY iconChanged)
    Q_PROPERTY(bool night READ night WRITE setNight NOTIFY iconChanged)

public:
    explicit DotIcon(QQuickItem *parent = nullptr);

    [[nodiscard]] QString condition() const { return m_condition; }
    [[nodiscard]] bool night() const { return m_night; }
    void setCondition(const QString &v);
    void setNight(bool v);

Q_SIGNALS:
    void iconChanged();

protected:
    void refill() override;

private:
    QString m_condition;
    bool m_night = false;
};
