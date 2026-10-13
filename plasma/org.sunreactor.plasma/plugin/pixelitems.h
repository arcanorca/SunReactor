#pragma once

#include <QColor>
#include <QImage>
#include <QQuickItem>
#include <QVariantList>
#include <QtQml/qqmlregistration.h>

/*
 * Pixel-art primitives (DESIGN.md section 4, pixel renderer rules).
 *
 * Measured on Qt 6.12 / RHI-GL at QT_SCALE_FACTOR 1.35 (tools/pixel_proof.sh):
 * any QML Text - plain, NativeRendering, or inside a layer whose texture is at
 * art resolution - is rasterised at the *window* device pixel ratio and then
 * resampled, which produces off-palette anti-aliasing colours. Rectangles and
 * nearest-filtered images at an integer device scale stay exact. So glyphs and
 * dither patterns are rasterised here, on the CPU, at art resolution (one image
 * pixel per art pixel, no anti-aliasing), and drawn as a texture with nearest
 * filtering at `artScale` logical px per art pixel. Inside a PixelSurface
 * (artScale 1, the surface's layer does the integer upscale) and on their own
 * (artScale = PixelGrid.unit) the result only ever contains the input colours.
 */

/*!
 * Device-pixel facts for pixel themes, per window.
 *
 * `devicePixelRatio` is the window's effective ratio (on Wayland with
 * fractional scaling that is the fractional value, which QScreen does not
 * report). One art pixel is `deviceScale` = k device pixels,
 * k = max(2, round(preferredScale * dpr)) with preferredScale 0 meaning 2, and
 * `unit` = k / dpr logical pixels.
 */
class PixelGrid : public QQuickItem
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(qreal devicePixelRatio READ devicePixelRatio NOTIFY gridChanged)
    /*! Preferred art-pixel size in logical px (theme pixel_scale); 0 = auto (2). */
    Q_PROPERTY(int preferredScale READ preferredScale WRITE setPreferredScale NOTIFY gridChanged)
    /*! Device pixels per art pixel: always an integer >= 2. */
    Q_PROPERTY(int deviceScale READ deviceScale NOTIFY gridChanged)
    /*! Logical pixels per art pixel (deviceScale / devicePixelRatio). */
    Q_PROPERTY(qreal unit READ unit NOTIFY gridChanged)
    /*! True when SUNREACTOR_HARNESS is set: surfaces report their geometry. */
    Q_PROPERTY(bool harness READ harness CONSTANT)

public:
    explicit PixelGrid(QQuickItem *parent = nullptr);

    [[nodiscard]] qreal devicePixelRatio() const { return m_dpr; }
    [[nodiscard]] int preferredScale() const { return m_preferredScale; }
    [[nodiscard]] int deviceScale() const;
    [[nodiscard]] qreal unit() const { return deviceScale() / m_dpr; }
    [[nodiscard]] bool harness() const;

    void setPreferredScale(int scale);

    /*! Rounds a logical length to whole device pixels. */
    Q_INVOKABLE qreal snap(qreal logical) const;
    /*! How many whole art pixels fit into a logical length. */
    Q_INVOKABLE int cells(qreal logical) const;
    /*! The item's bounding rect in window device pixels: {x, y, w, h}. */
    Q_INVOKABLE QVariantMap deviceRect(QQuickItem *item) const;

Q_SIGNALS:
    void gridChanged();

protected:
    void itemChange(ItemChange change, const ItemChangeData &value) override;

private:
    void updateRatio();

    qreal m_dpr = 1.0;
    int m_preferredScale = 0;
};

/*!
 * Base for CPU-rasterised pixel content: subclasses render one QImage at art
 * resolution; it is shown with nearest filtering, `artScale` logical px per
 * art px, top-left aligned.
 */
class PixelRaster : public QQuickItem
{
    Q_OBJECT
    QML_ANONYMOUS

    /*! Logical px per art px: 1 inside a PixelSurface, PixelGrid.unit outside. */
    Q_PROPERTY(qreal artScale READ artScale WRITE setArtScale NOTIFY artScaleChanged)
    Q_PROPERTY(int artWidth READ artWidth NOTIFY rasterChanged)
    Q_PROPERTY(int artHeight READ artHeight NOTIFY rasterChanged)

public:
    explicit PixelRaster(QQuickItem *parent = nullptr);

    [[nodiscard]] qreal artScale() const { return m_artScale; }
    void setArtScale(qreal scale);
    [[nodiscard]] int artWidth() const { return m_image.width(); }
    [[nodiscard]] int artHeight() const { return m_image.height(); }

Q_SIGNALS:
    void artScaleChanged();
    void rasterChanged();

protected:
    /*! Produce the art-resolution image; called lazily after invalidate(). */
    [[nodiscard]] virtual QImage render() const { return {}; }
    /*! Whether implicit size follows the rendered image (text) or not (fills). */
    [[nodiscard]] virtual bool sizesToContent() const { return true; }
    void invalidate();
    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *data) override;
    void geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry) override;
    void componentComplete() override;

private:
    void rerender();

    QImage m_image;
    qreal m_artScale = 1.0;
    bool m_imageDirty = true;
    bool m_textureDirty = true;
};

/*!
 * Text from a bitmap-grid font, rasterised 1:1 at the font's native pixel size
 * without anti-aliasing. `pixelSize` must be the font's grid (Press Start 2P:
 * 8, 16, 24; unscii-16: 16, 32) for crisp glyphs; any size yields only `color`.
 */
class PixelText : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QString text READ text WRITE setText NOTIFY textChanged)
    Q_PROPERTY(QColor color READ color WRITE setColor NOTIFY textChanged)
    Q_PROPERTY(QString fontFamily READ fontFamily WRITE setFontFamily NOTIFY textChanged)
    Q_PROPERTY(int pixelSize READ pixelSize WRITE setPixelSize NOTIFY textChanged)
    /*! Elide on the right beyond this many art px; 0 = never. */
    Q_PROPERTY(int maximumArtWidth READ maximumArtWidth WRITE setMaximumArtWidth NOTIFY textChanged)
    /*! Extra art px between glyphs (bitmap fonts often want 0 or 1). */
    Q_PROPERTY(int letterSpacing READ letterSpacing WRITE setLetterSpacing NOTIFY textChanged)

public:
    explicit PixelText(QQuickItem *parent = nullptr);

    [[nodiscard]] QString text() const { return m_text; }
    [[nodiscard]] QColor color() const { return m_color; }
    [[nodiscard]] QString fontFamily() const { return m_fontFamily; }
    [[nodiscard]] int pixelSize() const { return m_pixelSize; }
    [[nodiscard]] int maximumArtWidth() const { return m_maximumArtWidth; }
    [[nodiscard]] int letterSpacing() const { return m_letterSpacing; }
    void setText(const QString &text);
    void setColor(const QColor &color);
    void setFontFamily(const QString &family);
    void setPixelSize(int size);
    void setMaximumArtWidth(int width);
    void setLetterSpacing(int spacing);

Q_SIGNALS:
    void textChanged();

protected:
    [[nodiscard]] QImage render() const override;

private:
    QString m_text;
    QColor m_color = Qt::white;
    QString m_fontFamily;
    int m_pixelSize = 8;
    int m_maximumArtWidth = 0;
    int m_letterSpacing = 0;
};

/*!
 * Ordered (Bayer) dither across a colour ramp. `colors` is the ramp, dark to
 * light (usually Theme.shades); `level` 0..1 picks a position on it. With
 * `levelTo` >= 0 the level runs from `level` to `levelTo` along `orientation`.
 * Only ramp colours are ever written. Fills its own size (art size =
 * floor(size / artScale)).
 */
class PixelDither : public PixelRaster
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(QVariantList colors READ colors WRITE setColors NOTIFY ditherChanged)
    Q_PROPERTY(qreal level READ level WRITE setLevel NOTIFY ditherChanged)
    Q_PROPERTY(qreal levelTo READ levelTo WRITE setLevelTo NOTIFY ditherChanged)
    Q_PROPERTY(Qt::Orientation orientation READ orientation WRITE setOrientation NOTIFY ditherChanged)
    /*! "none", "bayer2" or "bayer4" (the theme's dither token). */
    Q_PROPERTY(QString pattern READ pattern WRITE setPattern NOTIFY ditherChanged)
    /*! Pattern phase in art px, to line patterns up across neighbouring items. */
    Q_PROPERTY(int phaseX READ phaseX WRITE setPhaseX NOTIFY ditherChanged)
    Q_PROPERTY(int phaseY READ phaseY WRITE setPhaseY NOTIFY ditherChanged)

public:
    explicit PixelDither(QQuickItem *parent = nullptr);

    [[nodiscard]] QVariantList colors() const { return m_colors; }
    [[nodiscard]] qreal level() const { return m_level; }
    [[nodiscard]] qreal levelTo() const { return m_levelTo; }
    [[nodiscard]] Qt::Orientation orientation() const { return m_orientation; }
    [[nodiscard]] QString pattern() const { return m_pattern; }
    [[nodiscard]] int phaseX() const { return m_phaseX; }
    [[nodiscard]] int phaseY() const { return m_phaseY; }
    void setColors(const QVariantList &colors);
    void setLevel(qreal level);
    void setLevelTo(qreal level);
    void setOrientation(Qt::Orientation orientation);
    void setPattern(const QString &pattern);
    void setPhaseX(int phase);
    void setPhaseY(int phase);

    /*! The ramp colour ordered dither picks at art pixel (x, y) for a level. */
    Q_INVOKABLE QColor colorAt(qreal level, int x, int y) const;

Q_SIGNALS:
    void ditherChanged();

protected:
    [[nodiscard]] QImage render() const override;
    [[nodiscard]] bool sizesToContent() const override { return false; }

private:
    [[nodiscard]] QRgb pick(qreal level, int x, int y, const QList<QRgb> &ramp) const;

    QVariantList m_colors;
    qreal m_level = 0.5;
    qreal m_levelTo = -1.0;
    Qt::Orientation m_orientation = Qt::Horizontal;
    QString m_pattern = QStringLiteral("bayer4");
    int m_phaseX = 0;
    int m_phaseY = 0;
};
