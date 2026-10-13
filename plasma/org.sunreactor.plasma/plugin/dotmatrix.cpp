#include "dotmatrix.h"

#include "glyphtables.h"

#include <QChar>
#include <QGuiApplication>
#include <QQuickWindow>
#include <QSGGeometry>
#include <QSGGeometryNode>
#include <QSGVertexColorMaterial>
#include <QVariantMap>

#include <algorithm>
#include <cmath>
#include <array>
#include <iterator>
#include <numbers>
#include <string_view>
#include <vector>

namespace
{
using GlyphTables::Face;
using GlyphTables::Glyph;

constexpr int kMinDevicePitch = 3;
constexpr int kMinSegments = 12;
constexpr int kMaxSegments = 48;
constexpr int kBlankAdvance = 3; // cells of an unknown character
constexpr int kIconSize = 12;

// Premultiplied colour (what the vertex colour material expects); 0 = not drawn.
QRgb premultiplied(const QColor &c)
{
    return c.isValid() ? qPremultiply(c.rgba()) : 0;
}

char32_t upperCase(char32_t c)
{
    if (c > 0xFFFF) {
        return c;
    }
    return QChar(static_cast<char16_t>(c)).toUpper().unicode();
}

const Glyph *findGlyph(const Face &face, char32_t c)
{
    const auto it = face.constFind(c);
    return it == face.constEnd() ? nullptr : &it.value();
}

// Code points of a string, as the glyph tables key them.
QList<char32_t> codePoints(const QString &s)
{
    const auto ucs4 = s.toUcs4();
    QList<char32_t> out;
    out.reserve(ucs4.size());
    for (const auto c : ucs4) {
        out.append(static_cast<char32_t>(c));
    }
    return out;
}

bool hasDescent(const Face &face, const QList<char32_t> &chars)
{
    for (const char32_t c : chars) {
        const Glyph *g = findGlyph(face, c);
        if (g && g->rows > 7) {
            return true;
        }
    }
    return false;
}

// Weather pictograms on a 12 x 12 grid, y down. '#' lit, '@' accent, '.' unlit.
struct IconBitmap {
    const char *name;
    std::array<const char *, kIconSize> rows;
};

constexpr IconBitmap kIcons[] = {
    {"clear", {".....##.....", ".#........#.", ".....##.....", "...#....#...", "............", "#.#......#.#", "#.#......#.#", "............", "...#....#...", ".....##.....", ".#........#.", ".....##....."}},
    {"clear_night", {"............", "....##......", "..##.#......", "..#..#......", ".#...#......", ".#...#......", ".#...#......", ".#....#.....", "..#....###..", "..##....##..", "....####....", "............"}},
    {"partly_cloudy", {"#......#....", "...##.......", "..#..#......", ".#....#.....", ".#....#####.", "..#..##....#", "...##.#....#", "#...##.....#", "....#......#", "....#......#", "....########", "............"}},
    {"partly_cloudy_night", {"............", ".##.........", ".##.........", "#.#.........", ".#.##..####.", ".#...##....#", "..###.#....#", "....##.....#", "....#......#", "....#......#", "....########", "............"}},
    {"cloudy", {"............", "............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", "............", "............", "............"}},
    {"drizzle", {"............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", "............", "............", "...#..#..#..", "............"}},
    {"rain", {"............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", "............", "...#..#..#..", "............", "....#..#..#."}},
    {"heavy_rain", {"............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", "............", "..#..#..#..#", "............", "...#..#..#.."}},
    {"thunderstorm", {"............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", ".......@....", "......@@....", ".....@@.....", "....@......."}},
    {"snow", {"............", ".....####...", "....#....#..", "....#....#..", "..##......#.", "..#........#", "..#........#", "..#########.", "............", "..#...#...#.", "....#...#...", ".#...#...#.."}},
    {"mist", {"............", "............", "............", ".#.#.#.#.#..", "............", "............", "..#.#.#.#.#.", "............", "............", ".#.#.#.#.#..", "............", "............"}},
    {"fog", {"............", "............", "............", ".##########.", "............", "............", ".##########.", "............", "............", ".##########.", "............", "............"}},
    {"atmospheric", {"..#......#..", ".....##.....", "....#..#....", "...#....#...", "...#....#...", "....#..#....", ".....##.....", "..#......#..", "............", ".#.#.#.#.#.#", "............", "#.#.#.#.#.#."}},
    {"unknown", {"............", "....###.....", "...#...#....", ".......#....", "......#.....", ".....#......", "............", ".....#......", "............", "............", "............", "............"}},
};
} // namespace

// --- DotField ----------------------------------------------------------------

DotField::DotField(QQuickItem *parent)
    : QQuickItem(parent)
{
    setFlag(ItemHasContents, true);
}

int DotField::devicePitch(qreal pitch, qreal dpr)
{
    return std::max(kMinDevicePitch, static_cast<int>(std::lround(pitch * dpr)));
}

qreal DotField::pitch() const
{
    return devicePitch(m_pitch, m_dpr) / m_dpr;
}

void DotField::setCols(int v)
{
    v = std::max(0, v);
    if (v == m_cols) {
        return;
    }
    setGrid(v, m_rows);
    Q_EMIT gridChanged();
    refresh();
}

void DotField::setRows(int v)
{
    v = std::max(0, v);
    if (v == m_rows) {
        return;
    }
    setGrid(m_cols, v);
    Q_EMIT gridChanged();
    refresh();
}

void DotField::setPitch(qreal v)
{
    v = std::max<qreal>(0.0, v);
    if (v == m_pitch) {
        return;
    }
    m_pitch = v;
    Q_EMIT pitchChanged();
    updateImplicitSize();
    markGeometry();
}

void DotField::setDotScale(qreal v)
{
    v = std::clamp<qreal>(v, 0.1, 1.0);
    if (v == m_dotScale) {
        return;
    }
    m_dotScale = v;
    Q_EMIT styleChanged();
    markGeometry();
}

void DotField::setLitColor(const QColor &v)
{
    if (v == m_lit) {
        return;
    }
    m_lit = v;
    Q_EMIT styleChanged();
    markGeometry();
}

void DotField::setAccentColor(const QColor &v)
{
    if (v == m_accent) {
        return;
    }
    m_accent = v;
    Q_EMIT styleChanged();
    markGeometry();
}

void DotField::setDimColor(const QColor &v)
{
    if (v == m_dim) {
        return;
    }
    m_dim = v;
    Q_EMIT styleChanged();
    markGeometry();
}

void DotField::setGhostColor(const QColor &v)
{
    if (v == m_ghost) {
        return;
    }
    m_ghost = v;
    Q_EMIT styleChanged();
    markGeometry();
}

void DotField::setCell(int col, int row, int cell)
{
    put(col, row, cell);
    markGeometry();
}

int DotField::cell(int col, int row) const
{
    if (col < 0 || row < 0 || col >= m_cols || row >= m_rows) {
        return Blank;
    }
    return m_cells[static_cast<size_t>(row) * static_cast<size_t>(m_cols) + static_cast<size_t>(col)];
}

void DotField::fill(int cell)
{
    std::fill(m_cells.begin(), m_cells.end(), static_cast<quint8>(std::clamp(cell, 0, static_cast<int>(Blank))));
    markGeometry();
}

void DotField::refresh()
{
    refill();
    updateImplicitSize();
    markGeometry();
}

void DotField::setGrid(int cols, int rows, int fill)
{
    m_cols = std::max(0, cols);
    m_rows = std::max(0, rows);
    m_cells.assign(static_cast<size_t>(m_cols) * static_cast<size_t>(m_rows),
                   static_cast<quint8>(std::clamp(fill, 0, static_cast<int>(Blank))));
}

void DotField::put(int col, int row, int cell)
{
    if (col < 0 || row < 0 || col >= m_cols || row >= m_rows) {
        return;
    }
    m_cells[static_cast<size_t>(row) * static_cast<size_t>(m_cols) + static_cast<size_t>(col)] =
        static_cast<quint8>(std::clamp(cell, 0, static_cast<int>(Blank)));
}

void DotField::updateImplicitSize()
{
    const qreal p = pitch();
    setImplicitSize(m_cols * p, m_rows * p);
}

void DotField::markGeometry()
{
    m_geometryDirty = true;
    update();
}

void DotField::updateDpr()
{
    qreal ratio = 1.0;
    if (window()) {
        ratio = window()->effectiveDevicePixelRatio();
    } else if (qGuiApp) {
        ratio = qGuiApp->devicePixelRatio();
    }
    if (ratio <= 0.0) {
        ratio = 1.0;
    }
    if (ratio == m_dpr) {
        return;
    }
    m_dpr = ratio;
    Q_EMIT pitchChanged();
    updateImplicitSize();
    markGeometry();
}

void DotField::itemChange(ItemChange change, const ItemChangeData &value)
{
    QQuickItem::itemChange(change, value);
    if (change == ItemSceneChange || change == ItemDevicePixelRatioHasChanged) {
        updateDpr();
    }
}

void DotField::componentComplete()
{
    QQuickItem::componentComplete();
    updateDpr();
    refresh();
}

QSGNode *DotField::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *)
{
    auto *node = static_cast<QSGGeometryNode *>(oldNode);
    if (node && !m_geometryDirty) {
        return node;
    }
    m_geometryDirty = false;

    const int pitch = devicePitch(m_pitch, m_dpr);
    // Diameter in device pixels, a whole number; centres sit on whole device pixels.
    const int dot = std::clamp(static_cast<int>(std::lround(pitch * m_dotScale)), 1, pitch);
    const int segments = std::clamp(dot * 2, kMinSegments, kMaxSegments);
    const QRgb colours[4] = {premultiplied(m_dim), premultiplied(m_lit), premultiplied(m_accent),
                             premultiplied(m_ghost)};

    size_t count = 0;
    for (const quint8 state : m_cells) {
        if (state < Blank && qAlpha(colours[state])) {
            ++count;
        }
    }
    if (count == 0) {
        delete node;
        return nullptr;
    }

    if (!node) {
        auto *geometry = new QSGGeometry(QSGGeometry::defaultAttributes_ColoredPoint2D(), 0, 0,
                                         QSGGeometry::UnsignedIntType);
        geometry->setDrawingMode(QSGGeometry::DrawTriangles);
        node = new QSGGeometryNode;
        node->setGeometry(geometry);
        node->setFlag(QSGNode::OwnsGeometry);
        node->setMaterial(new QSGVertexColorMaterial);
        node->setFlag(QSGNode::OwnsMaterial);
    }

    // Per dot: a centre, an inner ring at the core radius (full colour) and an
    // outer ring at one device pixel further (alpha 0): the colour fades across
    // the edge, which is the anti-aliasing.
    const size_t perDotVertices = 1 + 2 * static_cast<size_t>(segments);
    const size_t perDotIndices = 9 * static_cast<size_t>(segments);
    QSGGeometry *geometry = node->geometry();
    geometry->allocate(static_cast<int>(count * perDotVertices), static_cast<int>(count * perDotIndices));
    auto *vertices = geometry->vertexDataAsColoredPoint2D();
    quint32 *indices = geometry->indexDataAsUInt();

    const qreal rCore = (dot / 2.0 - 0.5) / m_dpr;
    const qreal rOut = (dot / 2.0 + 0.5) / m_dpr;
    const qreal step = 2.0 * std::numbers::pi / segments;

    size_t n = 0;
    for (int row = 0; row < m_rows; ++row) {
        for (int col = 0; col < m_cols; ++col) {
            const quint8 state = m_cells[static_cast<size_t>(row) * static_cast<size_t>(m_cols) + static_cast<size_t>(col)];
            if (state >= Blank || !qAlpha(colours[state])) {
                continue;
            }
            const QRgb c = colours[state];
            const auto r = static_cast<quint8>(qRed(c));
            const auto g = static_cast<quint8>(qGreen(c));
            const auto b = static_cast<quint8>(qBlue(c));
            const auto a = static_cast<quint8>(qAlpha(c));
            const int left = col * pitch + (pitch - dot) / 2;
            const int top = row * pitch + (pitch - dot) / 2;
            const qreal cx = (left + dot / 2.0) / m_dpr;
            const qreal cy = (top + dot / 2.0) / m_dpr;

            const auto base = static_cast<quint32>(n * perDotVertices);
            auto *v = vertices + n * perDotVertices;
            v[0].set(static_cast<float>(cx), static_cast<float>(cy), r, g, b, a);
            for (int k = 0; k < segments; ++k) {
                const qreal angle = step * k;
                const auto ca = static_cast<float>(std::cos(angle));
                const auto sa = static_cast<float>(std::sin(angle));
                v[1 + k].set(static_cast<float>(cx + rCore * ca), static_cast<float>(cy + rCore * sa), r, g, b, a);
                v[1 + segments + k].set(static_cast<float>(cx + rOut * ca), static_cast<float>(cy + rOut * sa), 0, 0, 0, 0);
            }

            quint32 *ix = indices + n * perDotIndices;
            for (int k = 0; k < segments; ++k) {
                const quint32 inner0 = base + 1 + static_cast<quint32>(k);
                const quint32 inner1 = base + 1 + static_cast<quint32>((k + 1) % segments);
                const quint32 outer0 = base + 1 + static_cast<quint32>(segments + k);
                const quint32 outer1 = base + 1 + static_cast<quint32>(segments + (k + 1) % segments);
                *ix++ = base;
                *ix++ = inner0;
                *ix++ = inner1;
                *ix++ = inner0;
                *ix++ = inner1;
                *ix++ = outer1;
                *ix++ = inner0;
                *ix++ = outer1;
                *ix++ = outer0;
            }
            ++n;
        }
    }
    node->markDirty(QSGNode::DirtyGeometry);
    return node;
}

// --- DotText -----------------------------------------------------------------

DotText::DotText(QQuickItem *parent)
    : DotField(parent)
{
}

bool DotText::tiny() const
{
    return m_face == QLatin1String("tiny");
}

int DotText::rowsPerGlyph() const
{
    return (tiny() ? 5 : 7) * m_size;
}

int DotText::descentRows() const
{
    if (tiny()) {
        return 0;
    }
    return hasDescent(GlyphTables::smallFace(), codePoints(m_text)) ? 2 * m_size : 0;
}

void DotText::setText(const QString &v)
{
    if (v == m_text) {
        return;
    }
    m_text = v;
    Q_EMIT textChanged();
    refresh();
}

void DotText::setSize(int v)
{
    v = std::clamp(v, 1, 8);
    if (v == m_size) {
        return;
    }
    m_size = v;
    Q_EMIT textChanged();
    refresh();
}

void DotText::setFace(const QString &v)
{
    const QString face = v == QLatin1String("tiny") ? QStringLiteral("tiny") : QStringLiteral("small");
    if (face == m_face) {
        return;
    }
    m_face = face;
    Q_EMIT textChanged();
    refresh();
}

void DotText::setLetterSpacing(int v)
{
    v = std::clamp(v, 0, 8);
    if (v == m_spacing) {
        return;
    }
    m_spacing = v;
    Q_EMIT textChanged();
    refresh();
}

void DotText::setAccentIndex(int v)
{
    if (v == m_accent) {
        return;
    }
    m_accent = v;
    Q_EMIT textChanged();
    refresh();
}

bool DotText::covers(const QString &text) const
{
    const Face &face = tiny() ? GlyphTables::tinyFace() : GlyphTables::smallFace();
    for (const char32_t c : text.toUcs4()) {
        if (!findGlyph(face, tiny() ? upperCase(c) : c)) {
            return false;
        }
    }
    return true;
}

void DotText::refill()
{
    const bool isTiny = tiny();
    const Face &face = isTiny ? GlyphTables::tinyFace() : GlyphTables::smallFace();
    const int capRows = isTiny ? 5 : 7;
    const QList<char32_t> chars = codePoints(m_text);

    std::vector<const Glyph *> glyphs;
    glyphs.reserve(static_cast<size_t>(chars.size()));
    int widthPx = 0;
    for (qsizetype i = 0; i < chars.size(); ++i) {
        const Glyph *g = findGlyph(face, isTiny ? upperCase(chars.at(i)) : chars.at(i));
        glyphs.push_back(g);
        if (i > 0) {
            widthPx += m_spacing;
        }
        widthPx += g ? g->width : kBlankAdvance;
    }
    const int boxRows = hasDescent(face, chars) ? capRows + 2 : capRows;
    setGrid(widthPx * m_size, boxRows * m_size, Blank);

    int x = 0;
    for (qsizetype i = 0; i < chars.size(); ++i) {
        if (i > 0) {
            x += m_spacing;
        }
        const Glyph *g = glyphs[static_cast<size_t>(i)];
        const int gw = g ? g->width : kBlankAdvance;
        if (g) {
            const int on = i == m_accent ? Accent : Lit;
            for (int gy = 0; gy < boxRows; ++gy) {
                const quint8 bits = g->bits[static_cast<size_t>(gy)];
                for (int gx = 0; gx < gw; ++gx) {
                    const bool lit = (bits & (1u << (gw - 1 - gx))) != 0;
                    for (int dy = 0; dy < m_size; ++dy) {
                        for (int dx = 0; dx < m_size; ++dx) {
                            put((x + gx) * m_size + dx, gy * m_size + dy, lit ? on : Off);
                        }
                    }
                }
            }
        }
        x += gw;
    }
}

// --- DotBar ------------------------------------------------------------------

DotBar::DotBar(QQuickItem *parent)
    : DotField(parent)
{
    setGrid(24, 1);
}

void DotBar::setValue(qreal v)
{
    v = std::clamp<qreal>(v, 0.0, 1.0);
    if (v == m_value) {
        return;
    }
    m_value = v;
    Q_EMIT barChanged();
    refresh();
}

void DotBar::setThumbAccent(bool v)
{
    if (v == m_thumbAccent) {
        return;
    }
    m_thumbAccent = v;
    Q_EMIT barChanged();
    refresh();
}

void DotBar::refill()
{
    const int n = cols();
    const int thumb = n > 1 ? static_cast<int>(std::lround(m_value * (n - 1))) : 0;
    for (int c = 0; c < n; ++c) {
        int state = Off;
        if (c < thumb) {
            state = Lit;
        } else if (c == thumb) {
            state = m_thumbAccent ? Accent : (m_value > 0.0 ? Lit : Off);
        }
        for (int r = 0; r < rows(); ++r) {
            put(c, r, state);
        }
    }
}

// --- DotRing -----------------------------------------------------------------

DotRing::DotRing(QQuickItem *parent)
    : DotField(parent)
{
    setGrid(m_diameter, m_diameter);
}

void DotRing::setValue(qreal v)
{
    v = std::clamp<qreal>(v, 0.0, 1.0);
    if (v == m_value) {
        return;
    }
    m_value = v;
    Q_EMIT ringChanged();
    refresh();
}

void DotRing::setDots(int v)
{
    v = std::clamp(v, 3, 240);
    if (v == m_dots) {
        return;
    }
    m_dots = v;
    Q_EMIT ringChanged();
    refresh();
}

void DotRing::setDiameter(int v)
{
    v = std::clamp(v, 3, 64);
    if (v == m_diameter) {
        return;
    }
    m_diameter = v;
    Q_EMIT ringChanged();
    refresh();
}

void DotRing::setHeadAccent(bool v)
{
    if (v == m_headAccent) {
        return;
    }
    m_headAccent = v;
    Q_EMIT ringChanged();
    refresh();
}

void DotRing::refill()
{
    const int d = m_diameter;
    setGrid(d, d, Blank);
    const qreal radius = (d - 1) / 2.0;
    const int n = m_dots;
    const int thumb = static_cast<int>(std::lround(m_value * (n - 1)));
    for (int k = 0; k < n; ++k) {
        const qreal angle = -std::numbers::pi / 2.0 + 2.0 * std::numbers::pi * k / n;
        const int col = static_cast<int>(std::lround(radius + radius * std::cos(angle)));
        const int row = static_cast<int>(std::lround(radius + radius * std::sin(angle)));
        if (cell(col, row) != Blank) {
            continue; // two points landed on one cell
        }
        int state = Off;
        if (k < thumb) {
            state = Lit;
        } else if (k == thumb) {
            state = m_headAccent ? Accent : (m_value > 0.0 ? Lit : Off);
        }
        put(col, row, state);
    }
}

// --- DotCurve ----------------------------------------------------------------

DotCurve::DotCurve(QQuickItem *parent)
    : DotField(parent)
{
    setGrid(48, 12);
}

std::vector<DotCurve::Sample> DotCurve::parse(const QVariantList &list)
{
    std::vector<Sample> out;
    out.reserve(static_cast<size_t>(list.size()));
    for (const QVariant &v : list) {
        const QVariantMap m = v.toMap();
        const qreal t = m.value(QStringLiteral("t")).toDouble();
        const QVariant pct = m.value(QStringLiteral("pct"));
        if (!pct.isValid() || (!out.empty() && t <= out.back().t)) {
            continue;
        }
        out.push_back({t, pct.toDouble()});
    }
    return out;
}

qreal DotCurve::interpolate(const std::vector<Sample> &s, qreal t)
{
    if (s.empty() || t < s.front().t || t > s.back().t) {
        return -1;
    }
    const auto it = std::lower_bound(s.begin(), s.end(), t, [](const Sample &a, qreal v) { return a.t < v; });
    if (it == s.begin()) {
        return it->pct;
    }
    const Sample &b = *it;
    const Sample &a = *(it - 1);
    const qreal f = b.t == a.t ? 0 : (t - a.t) / (b.t - a.t);
    return a.pct + (b.pct - a.pct) * f;
}

void DotCurve::setSamples(const QVariantList &v)
{
    if (v == m_samplesVar) {
        return;
    }
    m_samplesVar = v;
    m_samples = parse(v);
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::setSamplesNoWeather(const QVariantList &v)
{
    if (v == m_noWeatherVar) {
        return;
    }
    m_noWeatherVar = v;
    m_noWeather = parse(v);
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::setT0(qreal v)
{
    if (v == m_t0) {
        return;
    }
    m_t0 = v;
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::setT1(qreal v)
{
    if (v == m_t1) {
        return;
    }
    m_t1 = v;
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::setNowT(qreal v)
{
    if (v == m_nowT) {
        return;
    }
    m_nowT = v;
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::setMaxPct(qreal v)
{
    if (v == m_maxPct) {
        return;
    }
    m_maxPct = v;
    Q_EMIT curveChanged();
    refresh();
}

void DotCurve::refill()
{
    const int n = cols();
    const int r = std::max(2, rows());
    setGrid(n, r, Blank);
    if (n <= 0) {
        return;
    }
    const qreal start = m_t0 > 0 ? m_t0 : (m_samples.empty() ? 0 : m_samples.front().t);
    const qreal end = m_t1 > 0 ? m_t1 : (m_samples.empty() ? 1 : m_samples.back().t);
    const qreal span = std::max<qreal>(1, end - start);
    const qreal scaleTop = m_maxPct > 0 ? m_maxPct : 100.0;
    int nowCol = -1;
    if (m_nowT >= start && m_nowT <= end) {
        nowCol = std::min(n - 1, static_cast<int>(std::floor((m_nowT - start) / span * n)));
    }
    const auto rowFor = [r, scaleTop](qreal pct) {
        const qreal f = std::clamp<qreal>(pct / scaleTop, 0.0, 1.0);
        return static_cast<int>(std::lround((1.0 - f) * (r - 1)));
    };
    for (int x = 0; x < n; ++x) {
        const qreal t = start + (x + 0.5) / n * span;
        const qreal v = interpolate(m_samples, t);
        if (v < 0) {
            continue; // outside the day: blank column
        }
        const int top = rowFor(v);
        for (int y = 0; y < r; ++y) {
            put(x, y, y < top ? Off : (x == nowCol ? Accent : Lit));
        }
        if (!m_noWeather.empty()) {
            const qreal w = interpolate(m_noWeather, t);
            if (w >= 0) {
                const int outline = rowFor(w);
                if (outline < top) {
                    put(x, outline, Ghost);
                }
            }
        }
    }
}

// --- DotIcon -----------------------------------------------------------------

DotIcon::DotIcon(QQuickItem *parent)
    : DotField(parent)
{
    setGrid(kIconSize, kIconSize);
}

void DotIcon::setCondition(const QString &v)
{
    if (v == m_condition) {
        return;
    }
    m_condition = v;
    Q_EMIT iconChanged();
    refresh();
}

void DotIcon::setNight(bool v)
{
    if (v == m_night) {
        return;
    }
    m_night = v;
    Q_EMIT iconChanged();
    refresh();
}

void DotIcon::refill()
{
    setGrid(kIconSize, kIconSize, Off);
    QByteArray key = m_condition.toLatin1();
    if (m_night && (key == "clear" || key == "partly_cloudy")) {
        key += "_night";
    }
    // "unknown" is the last entry of the table.
    const IconBitmap *bitmap = &kIcons[std::size(kIcons) - 1];
    const std::string_view wanted(key.constData(), static_cast<size_t>(key.size()));
    for (const IconBitmap &icon : kIcons) {
        if (wanted == icon.name) {
            bitmap = &icon;
            break;
        }
    }
    for (int y = 0; y < kIconSize; ++y) {
        const char *row = bitmap->rows[static_cast<size_t>(y)];
        for (int x = 0; x < kIconSize; ++x) {
            const char ch = row[x];
            put(x, y, ch == '#' ? Lit : (ch == '@' ? Accent : Off));
        }
    }
}
