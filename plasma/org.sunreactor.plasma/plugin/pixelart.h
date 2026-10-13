#pragma once

#include "pixelitems.h"

#include <QColor>
#include <QImage>
#include <QUrl>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

#include <vector>

/*
 * Pixel-art primitives for the pixel skins (DESIGN.md section 4): everything
 * here is rasterised on the CPU at art resolution (one image pixel per art
 * pixel, no anti-aliasing) and shown by PixelRaster with nearest filtering, so
 * the output only ever contains the colours it was given. Images are rebuilt
 * only when a property changes; nothing runs per frame.
 */

/*!
 * Text and digits from built-in glyph faces that need no font file:
 *
 *   "small"  5x7 proportional, upper and lower case, digits, punctuation,
 *            Turkish and German letters (descenders below the 7-row cap);
 *   "tiny"   3x5 capitals, digits and the few symbols axis labels need;
 *   "seg"    seven-segment digits (and the letters a 7-segment cell can show),
 *            `segmentLength` x `segmentWidth`, unlit segments in ghostColor.
 *
 * `pitch` > 1 draws every glyph pixel as a `dot` x `dot` block on a pitch grid
 * (round when `round` and dot >= 3), unlit glyph pixels in ghostColor: the
 * dot-matrix / LCD-cell look. `covers()` says whether a face can show a string;
 * the QML wrapper falls back to the bitmap font when it cannot (translations).
 */
class PixelGlyphs : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString text READ text WRITE setText NOTIFY glyphsChanged)
    Q_PROPERTY(QColor color READ color WRITE setColor NOTIFY glyphsChanged)
    /*! Unlit cells / segments; transparent (default) draws nothing. */
    Q_PROPERTY(QColor ghostColor READ ghostColor WRITE setGhostColor NOTIFY glyphsChanged)
    Q_PROPERTY(QString face READ face WRITE setFace NOTIFY glyphsChanged)
    Q_PROPERTY(int pitch READ pitch WRITE setPitch NOTIFY glyphsChanged)
    /*! Lit block size inside the pitch; 0 = the whole pitch. */
    Q_PROPERTY(int dot READ dot WRITE setDot NOTIFY glyphsChanged)
    Q_PROPERTY(bool round READ round WRITE setRound NOTIFY glyphsChanged)
    Q_PROPERTY(int segmentLength READ segmentLength WRITE setSegmentLength NOTIFY glyphsChanged)
    Q_PROPERTY(int segmentWidth READ segmentWidth WRITE setSegmentWidth NOTIFY glyphsChanged)
    /*! Glyph pixels between glyphs (default 1). */
    Q_PROPERTY(int letterSpacing READ letterSpacing WRITE setLetterSpacing NOTIFY glyphsChanged)
    /*! Elide (single line) or wrap width (maximumLines > 1) in art px; 0 = none. */
    Q_PROPERTY(int maximumArtWidth READ maximumArtWidth WRITE setMaximumArtWidth NOTIFY glyphsChanged)
    Q_PROPERTY(int maximumLines READ maximumLines WRITE setMaximumLines NOTIFY glyphsChanged)
    Q_PROPERTY(int lineSpacing READ lineSpacing WRITE setLineSpacing NOTIFY glyphsChanged)
    /*! Qt::AlignLeft / AlignHCenter / AlignRight for wrapped lines. */
    Q_PROPERTY(int alignment READ alignment WRITE setAlignment NOTIFY glyphsChanged)
    /*! 50 % checker mask over the lit pixels: the 1-bit "greyed out" look. */
    Q_PROPERTY(bool stipple READ stipple WRITE setStipple NOTIFY glyphsChanged)
    Q_PROPERTY(bool capitals READ capitals WRITE setCapitals NOTIFY glyphsChanged)
    /*! Art px from the top of the image to the bottom of the capitals. */
    Q_PROPERTY(int capHeight READ capHeight NOTIFY glyphsChanged)
    Q_PROPERTY(int lineCount READ lineCount NOTIFY rasterChanged)
    Q_PROPERTY(bool elided READ elided NOTIFY rasterChanged)

public:
    explicit PixelGlyphs(QQuickItem *parent = nullptr);

    [[nodiscard]] QString text() const { return m_text; }
    [[nodiscard]] QColor color() const { return m_color; }
    [[nodiscard]] QColor ghostColor() const { return m_ghost; }
    [[nodiscard]] QString face() const { return m_face; }
    [[nodiscard]] int pitch() const { return m_pitch; }
    [[nodiscard]] int dot() const { return m_dot; }
    [[nodiscard]] bool round() const { return m_round; }
    [[nodiscard]] int segmentLength() const { return m_segLength; }
    [[nodiscard]] int segmentWidth() const { return m_segWidth; }
    [[nodiscard]] int letterSpacing() const { return m_letterSpacing; }
    [[nodiscard]] int maximumArtWidth() const { return m_maxWidth; }
    [[nodiscard]] int maximumLines() const { return m_maxLines; }
    [[nodiscard]] int lineSpacing() const { return m_lineSpacing; }
    [[nodiscard]] int alignment() const { return m_alignment; }
    [[nodiscard]] bool stipple() const { return m_stipple; }
    [[nodiscard]] bool capitals() const { return m_capitals; }
    [[nodiscard]] int capHeight() const;
    [[nodiscard]] int lineCount() const { return m_lineCount; }
    [[nodiscard]] bool elided() const { return m_elided; }

    void setText(const QString &v);
    void setColor(const QColor &v);
    void setGhostColor(const QColor &v);
    void setFace(const QString &v);
    void setPitch(int v);
    void setDot(int v);
    void setRound(bool v);
    void setSegmentLength(int v);
    void setSegmentWidth(int v);
    void setLetterSpacing(int v);
    void setMaximumArtWidth(int v);
    void setMaximumLines(int v);
    void setLineSpacing(int v);
    void setAlignment(int v);
    void setStipple(bool v);
    void setCapitals(bool v);

    /*! True when every character of `text` exists in `face`. */
    Q_INVOKABLE bool covers(const QString &text, const QString &face = {}) const;
    /*! Width in art px of one line of `text` with the current settings. */
    Q_INVOKABLE int measure(const QString &text) const;

Q_SIGNALS:
    void glyphsChanged();

protected:
    [[nodiscard]] QImage render() const override;

private:
    struct Line {
        QString text;
        int width = 0;
    };
    template<typename T>
    void assign(T &field, const T &value);
    [[nodiscard]] QString shown(const QString &text) const;
    [[nodiscard]] int advance(char32_t c) const;
    [[nodiscard]] int cellHeight() const;
    [[nodiscard]] int lineWidth(const QString &text) const;
    [[nodiscard]] std::vector<Line> layout(bool *elided) const;
    void drawGlyph(QImage &img, char32_t c, int x, int y, QRgb lit, QRgb ghost) const;

    QString m_text;
    QColor m_color = Qt::white;
    QColor m_ghost = Qt::transparent;
    QString m_face = QStringLiteral("small");
    int m_pitch = 1;
    int m_dot = 0;
    bool m_round = false;
    int m_segLength = 4;
    int m_segWidth = 1;
    int m_letterSpacing = 1;
    int m_maxWidth = 0;
    int m_maxLines = 1;
    int m_lineSpacing = 2;
    int m_alignment = Qt::AlignLeft;
    bool m_stipple = false;
    bool m_capitals = false;
    mutable int m_lineCount = 0;
    mutable bool m_elided = false;
};

/*!
 * Today's brightness curve rasterised into the art grid (the pixel LightCycle):
 * stepped line, patterned area under it, the weather-cost band between
 * pct_no_weather and pct, min/max limit lines, night shading, hour and sun
 * ticks, a now marker with the target and applied levels. Fills its own size
 * (art size = floor(size / artScale)); x is time from t0 to t1, y is 0..100 %.
 *
 * Patterns: "solid", "none", "bayer2", "bayer4" (with the level), "checker",
 * "hatch", "rhatch", "vlines", "hlines", "dots", "sparse".
 * `lattice` > 1 keeps only every lattice-th pixel in both directions (a dot
 * matrix) and fills the unlit lattice points with latticeColor; markers are
 * drawn after that so the now dot stays whole.
 */
class PixelCurve : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QVariantList samples READ samples WRITE setSamples NOTIFY curveChanged)
    /*! More sample lists drawn as plain lines (the "All displays" view). */
    Q_PROPERTY(QVariantList overlays READ overlays WRITE setOverlays NOTIFY curveChanged)
    Q_PROPERTY(QVariantList overlayColors READ overlayColors WRITE setOverlayColors NOTIFY curveChanged)
    Q_PROPERTY(qreal t0 READ t0 WRITE setT0 NOTIFY curveChanged)
    Q_PROPERTY(qreal t1 READ t1 WRITE setT1 NOTIFY curveChanged)
    Q_PROPERTY(qreal now READ now WRITE setNow NOTIFY curveChanged)
    Q_PROPERTY(int applied READ applied WRITE setApplied NOTIFY curveChanged)
    Q_PROPERTY(int minPct READ minPct WRITE setMinPct NOTIFY curveChanged)
    Q_PROPERTY(int maxPct READ maxPct WRITE setMaxPct NOTIFY curveChanged)
    Q_PROPERTY(qreal sunrise READ sunrise WRITE setSunrise NOTIFY curveChanged)
    Q_PROPERTY(qreal solarNoon READ solarNoon WRITE setSolarNoon NOTIFY curveChanged)
    Q_PROPERTY(qreal sunset READ sunset WRITE setSunset NOTIFY curveChanged)
    Q_PROPERTY(bool showArea READ showArea WRITE setShowArea NOTIFY curveChanged)

    Q_PROPERTY(QColor paperColor READ paperColor WRITE setPaperColor NOTIFY curveChanged)
    Q_PROPERTY(QColor lineColor READ lineColor WRITE setLineColor NOTIFY curveChanged)
    Q_PROPERTY(int lineWeight READ lineWeight WRITE setLineWeight NOTIFY curveChanged)
    Q_PROPERTY(QColor areaColor READ areaColor WRITE setAreaColor NOTIFY curveChanged)
    Q_PROPERTY(QString areaPattern READ areaPattern WRITE setAreaPattern NOTIFY curveChanged)
    Q_PROPERTY(qreal areaLevel READ areaLevel WRITE setAreaLevel NOTIFY curveChanged)
    Q_PROPERTY(QColor costColor READ costColor WRITE setCostColor NOTIFY curveChanged)
    Q_PROPERTY(QString costPattern READ costPattern WRITE setCostPattern NOTIFY curveChanged)
    Q_PROPERTY(QColor bandColor READ bandColor WRITE setBandColor NOTIFY curveChanged)
    Q_PROPERTY(QColor nightColor READ nightColor WRITE setNightColor NOTIFY curveChanged)
    Q_PROPERTY(QString nightPattern READ nightPattern WRITE setNightPattern NOTIFY curveChanged)
    Q_PROPERTY(QColor tickColor READ tickColor WRITE setTickColor NOTIFY curveChanged)
    Q_PROPERTY(QColor sunColor READ sunColor WRITE setSunColor NOTIFY curveChanged)
    Q_PROPERTY(QColor markerColor READ markerColor WRITE setMarkerColor NOTIFY curveChanged)
    Q_PROPERTY(QColor targetColor READ targetColor WRITE setTargetColor NOTIFY curveChanged)
    Q_PROPERTY(QColor appliedColor READ appliedColor WRITE setAppliedColor NOTIFY curveChanged)
    Q_PROPERTY(int lattice READ lattice WRITE setLattice NOTIFY curveChanged)
    Q_PROPERTY(QColor latticeColor READ latticeColor WRITE setLatticeColor NOTIFY curveChanged)
    /*! Round now dot (plus shape) instead of a square. */
    Q_PROPERTY(bool roundMarkers READ roundMarkers WRITE setRoundMarkers NOTIFY curveChanged)

public:
    explicit PixelCurve(QQuickItem *parent = nullptr);

    struct Sample {
        qreal t;
        qreal pct;
        qreal noWeather;
    };

    [[nodiscard]] QVariantList samples() const { return m_samplesVar; }
    [[nodiscard]] QVariantList overlays() const { return m_overlaysVar; }
    [[nodiscard]] QVariantList overlayColors() const { return m_overlayColors; }
    [[nodiscard]] qreal t0() const { return m_t0; }
    [[nodiscard]] qreal t1() const { return m_t1; }
    [[nodiscard]] qreal now() const { return m_now; }
    [[nodiscard]] int applied() const { return m_applied; }
    [[nodiscard]] int minPct() const { return m_minPct; }
    [[nodiscard]] int maxPct() const { return m_maxPct; }
    [[nodiscard]] qreal sunrise() const { return m_sunrise; }
    [[nodiscard]] qreal solarNoon() const { return m_noon; }
    [[nodiscard]] qreal sunset() const { return m_sunset; }
    [[nodiscard]] bool showArea() const { return m_showArea; }
    [[nodiscard]] QColor paperColor() const { return m_paper; }
    [[nodiscard]] QColor lineColor() const { return m_line; }
    [[nodiscard]] int lineWeight() const { return m_lineWeight; }
    [[nodiscard]] QColor areaColor() const { return m_area; }
    [[nodiscard]] QString areaPattern() const { return m_areaPattern; }
    [[nodiscard]] qreal areaLevel() const { return m_areaLevel; }
    [[nodiscard]] QColor costColor() const { return m_cost; }
    [[nodiscard]] QString costPattern() const { return m_costPattern; }
    [[nodiscard]] QColor bandColor() const { return m_band; }
    [[nodiscard]] QColor nightColor() const { return m_night; }
    [[nodiscard]] QString nightPattern() const { return m_nightPattern; }
    [[nodiscard]] QColor tickColor() const { return m_tick; }
    [[nodiscard]] QColor sunColor() const { return m_sun; }
    [[nodiscard]] QColor markerColor() const { return m_marker; }
    [[nodiscard]] QColor targetColor() const { return m_target; }
    [[nodiscard]] QColor appliedColor() const { return m_appliedColor; }
    [[nodiscard]] int lattice() const { return m_lattice; }
    [[nodiscard]] QColor latticeColor() const { return m_latticeColor; }
    [[nodiscard]] bool roundMarkers() const { return m_roundMarkers; }

    void setSamples(const QVariantList &v);
    void setOverlays(const QVariantList &v);
    void setOverlayColors(const QVariantList &v);
    void setT0(qreal v);
    void setT1(qreal v);
    void setNow(qreal v);
    void setApplied(int v);
    void setMinPct(int v);
    void setMaxPct(int v);
    void setSunrise(qreal v);
    void setSolarNoon(qreal v);
    void setSunset(qreal v);
    void setShowArea(bool v);
    void setPaperColor(const QColor &v);
    void setLineColor(const QColor &v);
    void setLineWeight(int v);
    void setAreaColor(const QColor &v);
    void setAreaPattern(const QString &v);
    void setAreaLevel(qreal v);
    void setCostColor(const QColor &v);
    void setCostPattern(const QString &v);
    void setBandColor(const QColor &v);
    void setNightColor(const QColor &v);
    void setNightPattern(const QString &v);
    void setTickColor(const QColor &v);
    void setSunColor(const QColor &v);
    void setMarkerColor(const QColor &v);
    void setTargetColor(const QColor &v);
    void setAppliedColor(const QColor &v);
    void setLattice(int v);
    void setLatticeColor(const QColor &v);
    void setRoundMarkers(bool v);

    /*! Art column of a time (may lie outside 0..artWidth-1). */
    Q_INVOKABLE int xAt(qreal t) const;
    /*! Art row of a percentage. */
    Q_INVOKABLE int yAt(qreal pct) const;
    /*! Time at the centre of an art column. */
    Q_INVOKABLE qreal timeAt(qreal artX) const;
    /*! Linear interpolation of the samples; -1 outside the day. */
    Q_INVOKABLE qreal valueAt(qreal t, bool withoutWeather = false) const;

    /*! Exposed for the pattern tests: whether `pattern` lights art px (x, y). */
    static bool patternOn(const QString &pattern, qreal level, int x, int y);

Q_SIGNALS:
    void curveChanged();

protected:
    [[nodiscard]] QImage render() const override;
    [[nodiscard]] bool sizesToContent() const override { return false; }

private:
    template<typename T>
    void assign(T &field, const T &value);
    [[nodiscard]] int artW() const;
    [[nodiscard]] int artH() const;
    [[nodiscard]] qreal start() const;
    [[nodiscard]] qreal end() const;
    static std::vector<Sample> parse(const QVariantList &list);
    static qreal interpolate(const std::vector<Sample> &s, qreal t, bool noWeather);

    QVariantList m_samplesVar;
    std::vector<Sample> m_samples;
    QVariantList m_overlaysVar;
    std::vector<std::vector<Sample>> m_overlays;
    QVariantList m_overlayColors;
    qreal m_t0 = 0;
    qreal m_t1 = 0;
    qreal m_now = 0;
    int m_applied = -1;
    int m_minPct = -1;
    int m_maxPct = -1;
    qreal m_sunrise = 0;
    qreal m_noon = 0;
    qreal m_sunset = 0;
    bool m_showArea = true;
    QColor m_paper = Qt::transparent;
    QColor m_line = Qt::white;
    int m_lineWeight = 1;
    QColor m_area = Qt::gray;
    QString m_areaPattern = QStringLiteral("bayer4");
    qreal m_areaLevel = 0.5;
    QColor m_cost = Qt::yellow;
    QString m_costPattern = QStringLiteral("checker");
    QColor m_band = Qt::gray;
    QColor m_night = Qt::transparent;
    QString m_nightPattern = QStringLiteral("sparse");
    QColor m_tick = Qt::gray;
    QColor m_sun = Qt::yellow;
    QColor m_marker = Qt::white;
    QColor m_target = Qt::red;
    QColor m_appliedColor = Qt::white;
    int m_lattice = 1;
    QColor m_latticeColor = Qt::transparent;
    bool m_roundMarkers = false;
};

/*!
 * One frame of an indexed PNG atlas, recoloured: atlas pixel index i becomes
 * colors[i] (index 0, a missing entry or a transparent colour stay
 * transparent). The atlases (icons/pixel/NAME.png) are generated by
 * tools/generate_weather_icons.py with tone indices, so a theme recolours them
 * with its own shades and the result passes the palette check by construction.
 */
class PixelSprite : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QUrl source READ source WRITE setSource NOTIFY spriteChanged)
    Q_PROPERTY(int frame READ frame WRITE setFrame NOTIFY spriteChanged)
    Q_PROPERTY(int frameWidth READ frameWidth WRITE setFrameWidth NOTIFY spriteChanged)
    Q_PROPERTY(int frameHeight READ frameHeight WRITE setFrameHeight NOTIFY spriteChanged)
    Q_PROPERTY(QVariantList colors READ colors WRITE setColors NOTIFY spriteChanged)
    Q_PROPERTY(bool mirror READ mirror WRITE setMirror NOTIFY spriteChanged)

public:
    explicit PixelSprite(QQuickItem *parent = nullptr);

    [[nodiscard]] QUrl source() const { return m_source; }
    [[nodiscard]] int frame() const { return m_frame; }
    [[nodiscard]] int frameWidth() const { return m_frameWidth; }
    [[nodiscard]] int frameHeight() const { return m_frameHeight; }
    [[nodiscard]] QVariantList colors() const { return m_colors; }
    [[nodiscard]] bool mirror() const { return m_mirror; }
    void setSource(const QUrl &v);
    void setFrame(int v);
    void setFrameWidth(int v);
    void setFrameHeight(int v);
    void setColors(const QVariantList &v);
    void setMirror(bool v);

Q_SIGNALS:
    void spriteChanged();

protected:
    [[nodiscard]] QImage render() const override;

private:
    QUrl m_source;
    int m_frame = 0;
    int m_frameWidth = 16;
    int m_frameHeight = 16;
    QVariantList m_colors;
    bool m_mirror = false;
};
