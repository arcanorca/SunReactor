#include "pixelitems.h"

#include <QFont>
#include <QFontMetrics>
#include <QGuiApplication>
#include <QPainter>
#include <QQuickWindow>
#include <QSGImageNode>
#include <QSGTexture>

#include <algorithm>
#include <cmath>

// --- PixelGrid ---------------------------------------------------------------

PixelGrid::PixelGrid(QQuickItem *parent)
    : QQuickItem(parent)
{
    updateRatio();
}

int PixelGrid::deviceScale() const
{
    const int preferred = m_preferredScale > 0 ? m_preferredScale : 2;
    return std::max(2, static_cast<int>(std::lround(preferred * m_dpr)));
}

bool PixelGrid::harness() const
{
    return qEnvironmentVariableIsSet("SUNREACTOR_HARNESS");
}

void PixelGrid::setPreferredScale(int scale)
{
    scale = std::clamp(scale, 0, 6);
    if (m_preferredScale == scale) {
        return;
    }
    m_preferredScale = scale;
    Q_EMIT gridChanged();
}

qreal PixelGrid::snap(qreal logical) const
{
    return std::round(logical * m_dpr) / m_dpr;
}

int PixelGrid::cells(qreal logical) const
{
    return std::max(0, static_cast<int>(std::floor(logical / unit() + 1e-6)));
}

QVariantMap PixelGrid::deviceRect(QQuickItem *item) const
{
    if (!item) {
        return {};
    }
    const QRectF scene = item->mapRectToScene(QRectF(0, 0, item->width(), item->height()));
    // Inward rounding: an edge that falls between device pixels belongs to
    // whichever side the rasteriser picked, so it is not part of the claim.
    const int x0 = static_cast<int>(std::ceil(scene.left() * m_dpr - 1e-6));
    const int y0 = static_cast<int>(std::ceil(scene.top() * m_dpr - 1e-6));
    const int x1 = static_cast<int>(std::floor(scene.right() * m_dpr + 1e-6));
    const int y1 = static_cast<int>(std::floor(scene.bottom() * m_dpr + 1e-6));
    return {{QStringLiteral("x"), x0},
            {QStringLiteral("y"), y0},
            {QStringLiteral("w"), std::max(0, x1 - x0)},
            {QStringLiteral("h"), std::max(0, y1 - y0)}};
}

void PixelGrid::itemChange(ItemChange change, const ItemChangeData &value)
{
    QQuickItem::itemChange(change, value);
    if (change == ItemSceneChange || change == ItemDevicePixelRatioHasChanged) {
        updateRatio();
    }
}

void PixelGrid::updateRatio()
{
    qreal ratio = window() ? window()->effectiveDevicePixelRatio() : qApp->devicePixelRatio();
    if (ratio <= 0.0) {
        ratio = 1.0;
    }
    if (!qFuzzyCompare(ratio, m_dpr)) {
        m_dpr = ratio;
        Q_EMIT gridChanged();
    }
}

// --- PixelRaster -------------------------------------------------------------

PixelRaster::PixelRaster(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
}

void PixelRaster::setArtScale(qreal scale)
{
    if (scale <= 0.0 || qFuzzyCompare(scale, m_artScale)) {
        return;
    }
    m_artScale = scale;
    Q_EMIT artScaleChanged();
    invalidate();
}

void PixelRaster::invalidate()
{
    m_imageDirty = true;
    // Render on the GUI thread right away: implicit sizes must be correct for
    // the next layout pass, and font rasterisation belongs on this thread.
    if (isComponentComplete()) {
        rerender();
    }
}

void PixelRaster::componentComplete()
{
    QQuickItem::componentComplete();
    rerender();
}

void PixelRaster::rerender()
{
    if (!m_imageDirty) {
        return;
    }
    m_imageDirty = false;
    const QSize before = m_image.size();
    m_image = render();
    m_textureDirty = true;
    if (sizesToContent()) {
        setImplicitSize(m_image.width() * m_artScale, m_image.height() * m_artScale);
    }
    if (m_image.size() != before) {
        Q_EMIT rasterChanged();
    }
    update();
}

void PixelRaster::geometryChange(const QRectF &newGeometry, const QRectF &oldGeometry)
{
    QQuickItem::geometryChange(newGeometry, oldGeometry);
    if (!sizesToContent() && newGeometry.size() != oldGeometry.size()) {
        invalidate();
    }
}

QSGNode *PixelRaster::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *)
{
    auto *node = static_cast<QSGImageNode *>(oldNode);
    if (m_image.isNull() || !window()) {
        delete node;
        return nullptr;
    }
    if (!node) {
        node = window()->createImageNode();
        node->setOwnsTexture(true);
        // The whole point: one art pixel becomes a block of identical device
        // pixels, never a blend of neighbours.
        node->setFiltering(QSGTexture::Nearest);
        node->setMipmapFiltering(QSGTexture::None);
        m_textureDirty = true;
    }
    if (m_textureDirty) {
        node->setTexture(window()->createTextureFromImage(m_image));
        m_textureDirty = false;
    }
    node->setSourceRect(QRectF(0, 0, m_image.width(), m_image.height()));
    node->setRect(QRectF(0, 0, m_image.width() * m_artScale, m_image.height() * m_artScale));
    return node;
}

// --- PixelText ---------------------------------------------------------------

PixelText::PixelText(QQuickItem *parent)
    : PixelRaster(parent)
{
}

void PixelText::setText(const QString &text)
{
    if (m_text == text) {
        return;
    }
    m_text = text;
    Q_EMIT textChanged();
    invalidate();
}

void PixelText::setColor(const QColor &color)
{
    if (m_color == color) {
        return;
    }
    m_color = color;
    Q_EMIT textChanged();
    invalidate();
}

void PixelText::setFontFamily(const QString &family)
{
    if (m_fontFamily == family) {
        return;
    }
    m_fontFamily = family;
    Q_EMIT textChanged();
    invalidate();
}

void PixelText::setPixelSize(int size)
{
    size = std::max(1, size);
    if (m_pixelSize == size) {
        return;
    }
    m_pixelSize = size;
    Q_EMIT textChanged();
    invalidate();
}

void PixelText::setMaximumArtWidth(int width)
{
    width = std::max(0, width);
    if (m_maximumArtWidth == width) {
        return;
    }
    m_maximumArtWidth = width;
    Q_EMIT textChanged();
    invalidate();
}

void PixelText::setLetterSpacing(int spacing)
{
    if (m_letterSpacing == spacing) {
        return;
    }
    m_letterSpacing = spacing;
    Q_EMIT textChanged();
    invalidate();
}

QImage PixelText::render() const
{
    if (m_text.isEmpty()) {
        return {};
    }
    QFont font(m_fontFamily);
    font.setPixelSize(m_pixelSize);
    font.setStyleStrategy(static_cast<QFont::StyleStrategy>(QFont::NoAntialias | QFont::NoSubpixelAntialias));
    font.setHintingPreference(QFont::PreferFullHinting);
    font.setKerning(false);
    if (m_letterSpacing != 0) {
        font.setLetterSpacing(QFont::AbsoluteSpacing, m_letterSpacing);
    }

    // Metrics against an image (dpr 1), never against the screen.
    QImage probe(1, 1, QImage::Format_ARGB32_Premultiplied);
    const QFontMetrics metrics(font, &probe);
    const QString shown = m_maximumArtWidth > 0
        ? metrics.elidedText(m_text, Qt::ElideRight, m_maximumArtWidth)
        : m_text;
    const int width = std::max(1, metrics.horizontalAdvance(shown));
    const int height = std::max(1, metrics.height());

    QImage image(width, height, QImage::Format_ARGB32_Premultiplied);
    image.fill(Qt::transparent);
    QPainter painter(&image);
    painter.setRenderHint(QPainter::Antialiasing, false);
    painter.setRenderHint(QPainter::TextAntialiasing, false);
    painter.setFont(font);
    painter.setPen(m_color);
    painter.drawText(0, metrics.ascent(), shown);
    painter.end();
    return image;
}

// --- PixelDither -------------------------------------------------------------

namespace
{
constexpr int kBayer2[2][2] = {{0, 2}, {3, 1}};
constexpr int kBayer4[4][4] = {{0, 8, 2, 10}, {12, 4, 14, 6}, {3, 11, 1, 9}, {15, 7, 13, 5}};
} // namespace

PixelDither::PixelDither(QQuickItem *parent)
    : PixelRaster(parent)
{
}

void PixelDither::setColors(const QVariantList &colors)
{
    if (m_colors == colors) {
        return;
    }
    m_colors = colors;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setLevel(qreal level)
{
    if (qFuzzyCompare(m_level, level)) {
        return;
    }
    m_level = level;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setLevelTo(qreal level)
{
    if (qFuzzyCompare(m_levelTo, level)) {
        return;
    }
    m_levelTo = level;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setOrientation(Qt::Orientation orientation)
{
    if (m_orientation == orientation) {
        return;
    }
    m_orientation = orientation;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setPattern(const QString &pattern)
{
    if (m_pattern == pattern) {
        return;
    }
    m_pattern = pattern;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setPhaseX(int phase)
{
    if (m_phaseX == phase) {
        return;
    }
    m_phaseX = phase;
    Q_EMIT ditherChanged();
    invalidate();
}

void PixelDither::setPhaseY(int phase)
{
    if (m_phaseY == phase) {
        return;
    }
    m_phaseY = phase;
    Q_EMIT ditherChanged();
    invalidate();
}

QRgb PixelDither::pick(qreal level, int x, int y, const QList<QRgb> &ramp) const
{
    const qsizetype n = ramp.size();
    if (n == 1) {
        return ramp.first();
    }
    const qreal v = std::clamp(level, 0.0, 1.0) * static_cast<qreal>(n - 1);
    const int base = std::min(static_cast<int>(std::floor(v)), static_cast<int>(n - 1));
    if (base >= n - 1) {
        return ramp.last();
    }
    const qreal frac = v - base;
    const int px = ((x + m_phaseX) % 4 + 4) % 4;
    const int py = ((y + m_phaseY) % 4 + 4) % 4;
    qreal threshold = 0.5;
    if (m_pattern == QLatin1String("bayer4")) {
        threshold = (kBayer4[py][px] + 0.5) / 16.0;
    } else if (m_pattern == QLatin1String("bayer2")) {
        threshold = (kBayer2[py % 2][px % 2] + 0.5) / 4.0;
    }
    return frac > threshold ? ramp.at(base + 1) : ramp.at(base);
}

QColor PixelDither::colorAt(qreal level, int x, int y) const
{
    QList<QRgb> ramp;
    for (const QVariant &c : m_colors) {
        ramp.append(c.value<QColor>().rgba());
    }
    if (ramp.isEmpty()) {
        return {};
    }
    return QColor::fromRgba(pick(level, x, y, ramp));
}

QImage PixelDither::render() const
{
    QList<QRgb> ramp;
    for (const QVariant &c : m_colors) {
        const QColor colour = c.value<QColor>();
        if (colour.isValid()) {
            ramp.append(qPremultiply(colour.rgba()));
        }
    }
    const int w = static_cast<int>(std::floor(width() / artScale() + 1e-6));
    const int h = static_cast<int>(std::floor(height() / artScale() + 1e-6));
    if (ramp.isEmpty() || w <= 0 || h <= 0) {
        return {};
    }
    QImage image(w, h, QImage::Format_ARGB32_Premultiplied);
    const bool gradient = m_levelTo >= 0.0;
    for (int y = 0; y < h; ++y) {
        auto *line = reinterpret_cast<QRgb *>(image.scanLine(y));
        for (int x = 0; x < w; ++x) {
            qreal level = m_level;
            if (gradient) {
                const qreal t = m_orientation == Qt::Horizontal ? (w > 1 ? qreal(x) / (w - 1) : 0.0)
                                                                : (h > 1 ? qreal(y) / (h - 1) : 0.0);
                level = m_level + (m_levelTo - m_level) * t;
            }
            line[x] = pick(level, x, y, ramp);
        }
    }
    return image;
}
