#include "pixelart.h"
#include "glyphtables.h"

#include <QFile>
#include <QHash>
#include <QMutex>
#include <QStringView>

#include <algorithm>
#include <array>
#include <cmath>

using namespace GlyphTables;

namespace
{
void fillRect(QImage &img, int x, int y, int w, int h, QRgb c)
{
    if (qAlpha(c) == 0) {
        return;
    }
    const int x0 = std::max(0, x);
    const int y0 = std::max(0, y);
    const int x1 = std::min(img.width(), x + w);
    const int y1 = std::min(img.height(), y + h);
    for (int yy = y0; yy < y1; ++yy) {
        auto *line = reinterpret_cast<QRgb *>(img.scanLine(yy));
        for (int xx = x0; xx < x1; ++xx) {
            line[xx] = c;
        }
    }
}

void plot(QImage &img, int x, int y, QRgb c)
{
    if (qAlpha(c) == 0 || x < 0 || y < 0 || x >= img.width() || y >= img.height()) {
        return;
    }
    reinterpret_cast<QRgb *>(img.scanLine(y))[x] = c;
}

QRgb premul(const QColor &c)
{
    return c.isValid() ? qPremultiply(c.rgba()) : 0;
}

QList<char32_t> codepoints(const QString &s)
{
    QList<char32_t> out;
    const auto ucs4 = s.toUcs4();
    out.reserve(ucs4.size());
    for (const char32_t c : ucs4) {
        out.append(c);
    }
    return out;
}
} // namespace

PixelGlyphs::PixelGlyphs(QQuickItem *parent)
    : PixelRaster(parent)
{
}

template<typename T>
void PixelGlyphs::assign(T &field, const T &value)
{
    if (field == value) {
        return;
    }
    field = value;
    Q_EMIT glyphsChanged();
    invalidate();
}

void PixelGlyphs::setText(const QString &v) { assign(m_text, v); }
void PixelGlyphs::setColor(const QColor &v) { assign(m_color, v); }
void PixelGlyphs::setGhostColor(const QColor &v) { assign(m_ghost, v); }
namespace {
/*! The built-in face a name selects; unknown names use the small face. */
const GlyphTables::Face &glyphFace(const QString &name)
{
    if (name == QLatin1String("tiny")) {
        return GlyphTables::tinyFace();
    }
    if (name == QLatin1String("topaz")) {
        return GlyphTables::topazFace();
    }
    if (name == QLatin1String("petscii")) {
        return GlyphTables::petsciiFace();
    }
    return GlyphTables::smallFace();
}
} // namespace

void PixelGlyphs::setFace(const QString &v) { assign(m_face, v); }
void PixelGlyphs::setPitch(int v) { assign(m_pitch, std::clamp(v, 1, 8)); }
void PixelGlyphs::setDot(int v) { assign(m_dot, std::clamp(v, 0, 8)); }
void PixelGlyphs::setRound(bool v) { assign(m_round, v); }
void PixelGlyphs::setSegmentLength(int v) { assign(m_segLength, std::clamp(v, 1, 32)); }
void PixelGlyphs::setSegmentWidth(int v) { assign(m_segWidth, std::clamp(v, 1, 8)); }
void PixelGlyphs::setLetterSpacing(int v) { assign(m_letterSpacing, std::clamp(v, 0, 16)); }
void PixelGlyphs::setMaximumArtWidth(int v) { assign(m_maxWidth, std::max(0, v)); }
void PixelGlyphs::setMaximumLines(int v) { assign(m_maxLines, std::clamp(v, 1, 8)); }
void PixelGlyphs::setLineSpacing(int v) { assign(m_lineSpacing, std::clamp(v, 0, 16)); }
void PixelGlyphs::setAlignment(int v) { assign(m_alignment, v); }
void PixelGlyphs::setStipple(bool v) { assign(m_stipple, v); }
void PixelGlyphs::setCapitals(bool v) { assign(m_capitals, v); }

int PixelGlyphs::capHeight() const
{
    if (m_face == QLatin1String("seg")) {
        return 2 * m_segLength + 3 * m_segWidth;
    }
    if (m_face == QLatin1String("petscii")) {
        return 8 * m_pitch;
    }
    return (m_face == QLatin1String("tiny") ? 5 : 7) * m_pitch;
}

int PixelGlyphs::cellHeight() const
{
    if (m_face == QLatin1String("seg")) {
        return capHeight();
    }
    if (m_face == QLatin1String("topaz") || m_face == QLatin1String("petscii")) {
        return 8 * m_pitch;
    }
    return (m_face == QLatin1String("tiny") ? 5 : 9) * m_pitch;
}

QString PixelGlyphs::shown(const QString &text) const
{
    return m_capitals ? text.toUpper() : text;
}

bool PixelGlyphs::covers(const QString &text, const QString &face) const
{
    const QString f = face.isEmpty() ? m_face : face;
    const QString s = m_capitals ? text.toUpper() : text;
    for (const char32_t c : codepoints(s)) {
        if (f == QLatin1String("seg")) {
            if (!segFace().contains(c) && !isSegNarrow(c)) {
                return false;
            }
        } else if (!glyphFace(f).contains(c)) {
            return false;
        }
    }
    return true;
}

int PixelGlyphs::advance(char32_t c) const
{
    if (m_face == QLatin1String("seg")) {
        const int w = isSegNarrow(c) ? m_segWidth : m_segLength + 2 * m_segWidth;
        return w + m_letterSpacing * m_segWidth + 1;
    }
    const Face &face = glyphFace(m_face);
    const auto it = face.constFind(c);
    const int w = it == face.constEnd() ? 3 : it->width;
    return (w + m_letterSpacing) * m_pitch;
}

int PixelGlyphs::lineWidth(const QString &text) const
{
    int w = 0;
    for (const char32_t c : codepoints(text)) {
        w += advance(c);
    }
    // No trailing gap after the last glyph.
    if (w > 0) {
        w -= m_face == QLatin1String("seg") ? (m_letterSpacing * m_segWidth + 1) : m_letterSpacing * m_pitch;
    }
    return std::max(0, w);
}

int PixelGlyphs::measure(const QString &text) const
{
    return lineWidth(shown(text));
}

std::vector<PixelGlyphs::Line> PixelGlyphs::layout(bool *elided) const
{
    *elided = false;
    const QString text = shown(m_text);
    std::vector<Line> lines;
    const auto elide = [this, elided](QString s) -> QString {
        const QString dots = QString(QChar(0x2026));
        if (m_maxWidth <= 0 || lineWidth(s) <= m_maxWidth) {
            return s;
        }
        *elided = true;
        while (!s.isEmpty() && lineWidth(QString(s + dots)) > m_maxWidth) {
            s.chop(1);
        }
        while (s.endsWith(QLatin1Char(' '))) {
            s.chop(1);
        }
        return QString(s + dots);
    };

    if (m_maxLines <= 1 || m_maxWidth <= 0) {
        const QString s = elide(text);
        lines.push_back({s, lineWidth(s)});
        return lines;
    }

    // Greedy word wrap; a word longer than the line is broken hard.
    QStringList words = text.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    QString current;
    qsizetype i = 0;
    while (i < words.size()) {
        const QString word = words.at(i);
        const QString candidate = current.isEmpty() ? word : current + QLatin1Char(' ') + word;
        if (lineWidth(candidate) <= m_maxWidth) {
            current = candidate;
            ++i;
            continue;
        }
        if (current.isEmpty()) {
            // Hard break inside the word.
            QString part = word;
            while (part.size() > 1 && lineWidth(part) > m_maxWidth) {
                part.chop(1);
            }
            current = part;
            if (static_cast<int>(lines.size()) == m_maxLines - 1) {
                break;
            }
            lines.push_back({current, lineWidth(current)});
            current.clear();
            words[i] = words.at(i).mid(part.size());
            continue;
        }
        if (static_cast<int>(lines.size()) == m_maxLines - 1) {
            break;
        }
        lines.push_back({current, lineWidth(current)});
        current.clear();
    }
    if (i < words.size()) {
        // Out of lines: put the rest on the last one and elide it.
        QStringList rest;
        for (qsizetype j = i; j < words.size(); ++j) {
            rest.append(words.at(j));
        }
        const QString all = current.isEmpty() ? rest.join(QLatin1Char(' '))
                                              : current + QLatin1Char(' ') + rest.join(QLatin1Char(' '));
        current = elide(all);
    }
    if (!current.isEmpty() || lines.empty()) {
        lines.push_back({current, lineWidth(current)});
    }
    return lines;
}

void PixelGlyphs::drawGlyph(QImage &img, char32_t c, int x, int y, QRgb lit, QRgb ghost) const
{
    if (m_face == QLatin1String("seg")) {
        const int T = m_segWidth;
        const int L = m_segLength;
        if (c == U':') {
            fillRect(img, x, y + T + (L - T) / 2, T, T, lit);
            fillRect(img, x, y + 2 * T + L + (L - T) / 2, T, T, lit);
            return;
        }
        if (c == U'.') {
            fillRect(img, x, y + 2 * L + 2 * T, T, T, lit);
            return;
        }
        const quint8 bits = segFace().value(c, 0);
        struct R {
            int x, y, w, h;
        };
        const R segs[7] = {
            {T, 0, L, T},                 // a
            {T + L, T, T, L},             // b
            {T + L, 2 * T + L, T, L},     // c
            {T, 2 * L + 2 * T, L, T},     // d
            {0, 2 * T + L, T, L},         // e
            {0, T, T, L},                 // f
            {T, T + L, L, T},             // g
        };
        for (int s = 0; s < 7; ++s) {
            const QRgb colour = (bits & (1 << s)) ? lit : ghost;
            fillRect(img, x + segs[s].x, y + segs[s].y, segs[s].w, segs[s].h, colour);
        }
        return;
    }

    const Face &face = glyphFace(m_face);
    const auto it = face.constFind(c);
    if (it == face.constEnd()) {
        return;
    }
    const Glyph &g = *it;
    const int p = m_pitch;
    const int d = m_dot > 0 ? std::min(m_dot, p) : p;
    const int rows = m_face == QLatin1String("tiny") ? 5 : (g.rows > 7 ? g.rows : 7);
    const bool ghostCells = qAlpha(ghost) != 0;
    for (int gy = 0; gy < rows; ++gy) {
        const quint8 bits = g.bits[static_cast<size_t>(gy)];
        // Ghost cells only inside the capital box (not under descenders).
        if (!bits && !(ghostCells && gy < (m_face == QLatin1String("tiny") ? 5 : 7))) {
            continue;
        }
        for (int gx = 0; gx < g.width; ++gx) {
            const bool on = bits & (1u << (g.width - 1 - gx));
            if (!on && (!ghostCells || gy >= (m_face == QLatin1String("tiny") ? 5 : 7))) {
                continue;
            }
            const QRgb colour = on ? lit : ghost;
            const int bx = x + gx * p;
            const int by = y + gy * p;
            for (int yy = 0; yy < d; ++yy) {
                for (int xx = 0; xx < d; ++xx) {
                    if (m_round && d >= 3 && (xx == 0 || xx == d - 1) && (yy == 0 || yy == d - 1)) {
                        continue;
                    }
                    if (on && m_stipple && ((bx + xx + by + yy) & 1)) {
                        continue;
                    }
                    plot(img, bx + xx, by + yy, colour);
                }
            }
        }
    }
}

QImage PixelGlyphs::render() const
{
    if (m_text.isEmpty()) {
        m_lineCount = 0;
        m_elided = false;
        return {};
    }
    bool elided = false;
    const std::vector<Line> lines = layout(&elided);
    m_elided = elided;
    m_lineCount = static_cast<int>(lines.size());
    int width = 0;
    for (const Line &l : lines) {
        width = std::max(width, l.width);
    }
    if (m_maxLines > 1 && m_maxWidth > 0 && m_alignment != Qt::AlignLeft) {
        width = std::max(width, m_maxWidth);
    }
    const int cell = cellHeight();
    const int height = cell * m_lineCount + m_lineSpacing * (m_lineCount - 1);
    if (width <= 0 || height <= 0) {
        return {};
    }
    QImage img(width, height, QImage::Format_ARGB32_Premultiplied);
    img.fill(Qt::transparent);
    const QRgb lit = premul(m_color);
    const QRgb ghost = premul(m_ghost);
    int y = 0;
    for (const Line &line : lines) {
        int x = 0;
        if (m_alignment & Qt::AlignHCenter) {
            x = (width - line.width) / 2;
        } else if (m_alignment & Qt::AlignRight) {
            x = width - line.width;
        }
        for (const char32_t c : codepoints(line.text)) {
            drawGlyph(img, c, x, y, lit, ghost);
            x += advance(c);
        }
        y += cell + m_lineSpacing;
    }
    return img;
}

// --- PixelCurve ----------------------------------------------------------------

PixelCurve::PixelCurve(QQuickItem *parent)
    : PixelRaster(parent)
{
}

template<typename T>
void PixelCurve::assign(T &field, const T &value)
{
    if (field == value) {
        return;
    }
    field = value;
    Q_EMIT curveChanged();
    invalidate();
}

std::vector<PixelCurve::Sample> PixelCurve::parse(const QVariantList &list)
{
    std::vector<Sample> out;
    out.reserve(static_cast<size_t>(list.size()));
    for (const QVariant &v : list) {
        const QVariantMap m = v.toMap();
        const qreal t = m.value(QStringLiteral("t")).toDouble();
        const qreal pct = m.value(QStringLiteral("pct")).toDouble();
        const QVariant nw = m.value(QStringLiteral("pctNoWeather"));
        if (t <= 0 || (!out.empty() && t <= out.back().t)) {
            continue;
        }
        out.push_back({t, pct, nw.isValid() ? nw.toDouble() : pct});
    }
    return out;
}

void PixelCurve::setSamples(const QVariantList &v)
{
    if (m_samplesVar == v) {
        return;
    }
    m_samplesVar = v;
    m_samples = parse(v);
    Q_EMIT curveChanged();
    invalidate();
}

void PixelCurve::setOverlays(const QVariantList &v)
{
    if (m_overlaysVar == v) {
        return;
    }
    m_overlaysVar = v;
    m_overlays.clear();
    for (const QVariant &list : v) {
        m_overlays.push_back(parse(list.toList()));
    }
    Q_EMIT curveChanged();
    invalidate();
}

void PixelCurve::setOverlayColors(const QVariantList &v) { assign(m_overlayColors, v); }
void PixelCurve::setT0(qreal v) { assign(m_t0, v); }
void PixelCurve::setT1(qreal v) { assign(m_t1, v); }
void PixelCurve::setNow(qreal v) { assign(m_now, v); }
void PixelCurve::setApplied(int v) { assign(m_applied, v); }
void PixelCurve::setMinPct(int v) { assign(m_minPct, v); }
void PixelCurve::setMaxPct(int v) { assign(m_maxPct, v); }
void PixelCurve::setSunrise(qreal v) { assign(m_sunrise, v); }
void PixelCurve::setSolarNoon(qreal v) { assign(m_noon, v); }
void PixelCurve::setSunset(qreal v) { assign(m_sunset, v); }
void PixelCurve::setShowArea(bool v) { assign(m_showArea, v); }
void PixelCurve::setPaperColor(const QColor &v) { assign(m_paper, v); }
void PixelCurve::setLineColor(const QColor &v) { assign(m_line, v); }
void PixelCurve::setLineWeight(int v) { assign(m_lineWeight, std::clamp(v, 1, 3)); }
void PixelCurve::setAreaColor(const QColor &v) { assign(m_area, v); }
void PixelCurve::setAreaPattern(const QString &v) { assign(m_areaPattern, v); }
void PixelCurve::setAreaLevel(qreal v) { assign(m_areaLevel, v); }
void PixelCurve::setCostColor(const QColor &v) { assign(m_cost, v); }
void PixelCurve::setCostPattern(const QString &v) { assign(m_costPattern, v); }
void PixelCurve::setBandColor(const QColor &v) { assign(m_band, v); }
void PixelCurve::setNightColor(const QColor &v) { assign(m_night, v); }
void PixelCurve::setNightPattern(const QString &v) { assign(m_nightPattern, v); }
void PixelCurve::setTickColor(const QColor &v) { assign(m_tick, v); }
void PixelCurve::setSunColor(const QColor &v) { assign(m_sun, v); }
void PixelCurve::setMarkerColor(const QColor &v) { assign(m_marker, v); }
void PixelCurve::setTargetColor(const QColor &v) { assign(m_target, v); }
void PixelCurve::setAppliedColor(const QColor &v) { assign(m_appliedColor, v); }
void PixelCurve::setLattice(int v) { assign(m_lattice, std::clamp(v, 1, 4)); }
void PixelCurve::setLatticeColor(const QColor &v) { assign(m_latticeColor, v); }
void PixelCurve::setRoundMarkers(bool v) { assign(m_roundMarkers, v); }

int PixelCurve::artW() const
{
    return std::max(0, static_cast<int>(std::floor(width() / artScale() + 1e-6)));
}

int PixelCurve::artH() const
{
    return std::max(0, static_cast<int>(std::floor(height() / artScale() + 1e-6)));
}

qreal PixelCurve::start() const
{
    return m_t0 > 0 ? m_t0 : (m_samples.empty() ? 0 : m_samples.front().t);
}

qreal PixelCurve::end() const
{
    return m_t1 > 0 ? m_t1 : (m_samples.empty() ? 1 : m_samples.back().t);
}

int PixelCurve::xAt(qreal t) const
{
    const qreal span = std::max<qreal>(1, end() - start());
    return static_cast<int>(std::floor((t - start()) / span * artW()));
}

int PixelCurve::yAt(qreal pct) const
{
    const int h = artH();
    const qreal p = std::clamp<qreal>(pct, 0, 100) / 100.0;
    return static_cast<int>(std::lround((1.0 - p) * (h - 1)));
}

qreal PixelCurve::timeAt(qreal artX) const
{
    const int w = std::max(1, artW());
    return start() + (artX + 0.5) / w * (end() - start());
}

qreal PixelCurve::interpolate(const std::vector<Sample> &s, qreal t, bool noWeather)
{
    if (s.empty() || t < s.front().t || t > s.back().t) {
        return -1;
    }
    const auto it = std::lower_bound(s.begin(), s.end(), t, [](const Sample &a, qreal v) { return a.t < v; });
    if (it == s.begin()) {
        return noWeather ? it->noWeather : it->pct;
    }
    const Sample &b = *it;
    const Sample &a = *(it - 1);
    const qreal f = b.t == a.t ? 0 : (t - a.t) / (b.t - a.t);
    const qreal va = noWeather ? a.noWeather : a.pct;
    const qreal vb = noWeather ? b.noWeather : b.pct;
    return va + (vb - va) * f;
}

qreal PixelCurve::valueAt(qreal t, bool withoutWeather) const
{
    return interpolate(m_samples, t, withoutWeather);
}

bool PixelCurve::patternOn(const QString &pattern, qreal level, int x, int y)
{
    static constexpr int bayer4[4][4] = {{0, 8, 2, 10}, {12, 4, 14, 6}, {3, 11, 1, 9}, {15, 7, 13, 5}};
    static constexpr int bayer2[2][2] = {{0, 2}, {3, 1}};
    const int px = ((x % 4) + 4) % 4;
    const int py = ((y % 4) + 4) % 4;
    if (pattern == QLatin1String("solid")) {
        return true;
    }
    if (pattern == QLatin1String("bayer4")) {
        return (bayer4[py][px] + 0.5) / 16.0 < level;
    }
    if (pattern == QLatin1String("bayer2")) {
        return (bayer2[py % 2][px % 2] + 0.5) / 4.0 < level;
    }
    if (pattern == QLatin1String("checker")) {
        return ((x + y) & 1) == 0;
    }
    if (pattern == QLatin1String("hatch")) {
        return (((x + y) % 4) + 4) % 4 == 0;
    }
    if (pattern == QLatin1String("rhatch")) {
        return (((x - y) % 4) + 4) % 4 == 0;
    }
    if (pattern == QLatin1String("vlines")) {
        return (x & 1) == 0;
    }
    if (pattern == QLatin1String("hlines")) {
        return (y & 1) == 0;
    }
    if (pattern == QLatin1String("dots")) {
        return (x & 1) == 0 && (y & 1) == 0;
    }
    if (pattern == QLatin1String("sparse")) {
        return px == 0 && py == 0;
    }
    return false;
}

QImage PixelCurve::render() const
{
    const int w = artW();
    const int h = artH();
    if (w <= 1 || h <= 2) {
        return {};
    }
    QImage img(w, h, QImage::Format_ARGB32_Premultiplied);
    img.fill(m_paper.isValid() ? qPremultiply(m_paper.rgba()) : 0);

    const QRgb line = premul(m_line);
    const QRgb area = premul(m_area);
    const QRgb cost = premul(m_cost);
    const QRgb band = premul(m_band);
    const QRgb night = premul(m_night);
    const QRgb tick = premul(m_tick);
    const QRgb sun = premul(m_sun);

    // Night: the sky before sunrise and after sunset.
    if (qAlpha(night) && m_sunrise > 0 && m_sunset > m_sunrise) {
        for (int x = 0; x < w; ++x) {
            const qreal t = timeAt(x);
            if (t >= m_sunrise && t <= m_sunset) {
                continue;
            }
            for (int y = 0; y < h; ++y) {
                if (patternOn(m_nightPattern, 0.5, x, y)) {
                    plot(img, x, y, night);
                }
            }
        }
    }

    // Limits band: dashed min and max lines.
    if (qAlpha(band)) {
        for (const int pct : {m_minPct, m_maxPct}) {
            if (pct < 0) {
                continue;
            }
            const int y = yAt(pct);
            for (int x = 0; x < w; ++x) {
                if ((x % 4) < 2) {
                    plot(img, x, y, band);
                }
            }
        }
    }

    // Hour ticks on the bottom edge: 1 px every 3 h, 2 px every 6 h.
    if (qAlpha(tick) && end() - start() > 3600) {
        for (int hour = 3; hour < 24; hour += 3) {
            const int x = xAt(start() + hour * 3600.0);
            if (x <= 0 || x >= w) {
                continue;
            }
            const int len = hour % 6 == 0 ? 2 : 1;
            for (int i = 0; i < len; ++i) {
                plot(img, x, h - 1 - i, tick);
            }
        }
    }

    // Columns of the primary curve.
    std::vector<int> ys(static_cast<size_t>(w), -1);
    std::vector<int> yn(static_cast<size_t>(w), -1);
    for (int x = 0; x < w; ++x) {
        const qreal t = timeAt(x);
        const qreal v = interpolate(m_samples, t, false);
        if (v < 0) {
            continue;
        }
        ys[static_cast<size_t>(x)] = yAt(v);
        yn[static_cast<size_t>(x)] = yAt(interpolate(m_samples, t, true));
    }

    for (int x = 0; x < w; ++x) {
        const int y = ys[static_cast<size_t>(x)];
        if (y < 0) {
            continue;
        }
        if (m_showArea && qAlpha(area)) {
            for (int yy = y + 1; yy < h; ++yy) {
                if (patternOn(m_areaPattern, m_areaLevel, x, yy)) {
                    plot(img, x, yy, area);
                }
            }
        }
        // What the weather costs: between pct_no_weather and pct.
        const int top = yn[static_cast<size_t>(x)];
        if (m_showArea && qAlpha(cost) && top >= 0 && top < y) {
            for (int yy = top; yy < y; ++yy) {
                if (patternOn(m_costPattern, 0.5, x, yy) || yy == top) {
                    plot(img, x, yy, cost);
                }
            }
        }
    }

    // Lines: stepped, each column joined vertically to the previous one.
    const auto drawLine = [&](const std::vector<int> &col, QRgb colour) {
        int prev = -1;
        for (int x = 0; x < w; ++x) {
            const int y = col[static_cast<size_t>(x)];
            if (y < 0) {
                prev = -1;
                continue;
            }
            const int a = prev < 0 ? y : std::min(prev, y);
            const int b = prev < 0 ? y : std::max(prev, y);
            for (int yy = a; yy <= b; ++yy) {
                for (int k = 0; k < m_lineWeight; ++k) {
                    plot(img, x, yy - k, colour);
                }
            }
            prev = y;
        }
    };
    for (size_t i = 0; i < m_overlays.size(); ++i) {
        std::vector<int> col(static_cast<size_t>(w), -1);
        for (int x = 0; x < w; ++x) {
            const qreal v = interpolate(m_overlays[i], timeAt(x), false);
            if (v >= 0) {
                col[static_cast<size_t>(x)] = yAt(v);
            }
        }
        const QColor c = i < static_cast<size_t>(m_overlayColors.size())
            ? m_overlayColors.at(static_cast<qsizetype>(i)).value<QColor>()
            : m_line;
        drawLine(col, premul(c));
    }
    drawLine(ys, line);

    // Sunrise / noon / sunset: a 3 px tick hanging from the top edge, and a
    // dotted guide down to the curve.
    if (qAlpha(sun)) {
        for (const qreal t : {m_sunrise, m_noon, m_sunset}) {
            if (t <= 0) {
                continue;
            }
            const int x = xAt(t);
            if (x < 0 || x >= w) {
                continue;
            }
            const int stop = ys[static_cast<size_t>(x)] >= 0 ? ys[static_cast<size_t>(x)] - 1 : h - 1;
            for (int y = 0; y < stop; ++y) {
                if (y < 3 || y % 3 == 0) {
                    plot(img, x, y, sun);
                }
            }
        }
    }

    // Dot matrix: keep the lattice, light the unlit lattice points.
    if (m_lattice > 1) {
        const QRgb paper = m_paper.isValid() ? qPremultiply(m_paper.rgba()) : 0;
        const QRgb ghost = premul(m_latticeColor);
        for (int y = 0; y < h; ++y) {
            auto *row = reinterpret_cast<QRgb *>(img.scanLine(y));
            for (int x = 0; x < w; ++x) {
                const bool onLattice = (x % m_lattice) == 0 && (y % m_lattice) == 0;
                if (!onLattice) {
                    row[x] = paper;
                } else if (row[x] == paper && qAlpha(ghost)) {
                    row[x] = ghost;
                }
            }
        }
    }

    // Now: dotted marker, applied level (hollow box), target (solid dot).
    if (m_now > 0) {
        const int x = xAt(m_now);
        if (x >= 0 && x < w) {
            const QRgb marker = premul(m_marker);
            for (int y = 0; y < h; y += 2) {
                plot(img, x, y, marker);
            }
            if (m_applied >= 0) {
                const int y = yAt(m_applied);
                const QRgb c = premul(m_appliedColor);
                for (int i = -2; i <= 2; ++i) {
                    plot(img, x + i, y - 2, c);
                    plot(img, x + i, y + 2, c);
                    plot(img, x - 2, y + i, c);
                    plot(img, x + 2, y + i, c);
                }
            }
            const qreal target = interpolate(m_samples, m_now, false);
            if (target >= 0) {
                const int y = yAt(target);
                const QRgb c = premul(m_target);
                for (int dy = -1; dy <= 1; ++dy) {
                    for (int dx = -1; dx <= 1; ++dx) {
                        if (m_roundMarkers && dx != 0 && dy != 0) {
                            continue;
                        }
                        plot(img, x + dx, y + dy, c);
                    }
                }
            }
        }
    }
    return img;
}

// --- PixelSprite -----------------------------------------------------------------

namespace
{
QImage loadAtlas(const QUrl &url)
{
    static QMutex mutex;
    static QHash<QString, QImage> cache;
    const QString path = url.isLocalFile() ? url.toLocalFile()
                                           : (url.scheme() == QLatin1String("qrc") ? QLatin1Char(':') + url.path()
                                                                                   : url.toString());
    const QMutexLocker lock(&mutex);
    const auto it = cache.constFind(path);
    if (it != cache.constEnd()) {
        return *it;
    }
    QImage image(path);
    if (!image.isNull() && image.format() != QImage::Format_Indexed8) {
        // Only palette-index atlases carry tone indices; anything else is refused.
        qWarning("PixelSprite: %s is not an indexed PNG", qPrintable(path));
        image = QImage();
    }
    cache.insert(path, image);
    return image;
}
} // namespace

PixelSprite::PixelSprite(QQuickItem *parent)
    : PixelRaster(parent)
{
}

void PixelSprite::setSource(const QUrl &v)
{
    if (m_source == v) {
        return;
    }
    m_source = v;
    Q_EMIT spriteChanged();
    invalidate();
}

void PixelSprite::setFrame(int v)
{
    if (m_frame == v) {
        return;
    }
    m_frame = v;
    Q_EMIT spriteChanged();
    invalidate();
}

void PixelSprite::setFrameWidth(int v)
{
    v = std::max(1, v);
    if (m_frameWidth == v) {
        return;
    }
    m_frameWidth = v;
    Q_EMIT spriteChanged();
    invalidate();
}

void PixelSprite::setFrameHeight(int v)
{
    v = std::max(1, v);
    if (m_frameHeight == v) {
        return;
    }
    m_frameHeight = v;
    Q_EMIT spriteChanged();
    invalidate();
}

void PixelSprite::setColors(const QVariantList &v)
{
    if (m_colors == v) {
        return;
    }
    m_colors = v;
    Q_EMIT spriteChanged();
    invalidate();
}

void PixelSprite::setMirror(bool v)
{
    if (m_mirror == v) {
        return;
    }
    m_mirror = v;
    Q_EMIT spriteChanged();
    invalidate();
}

QImage PixelSprite::render() const
{
    if (m_source.isEmpty()) {
        return {};
    }
    const QImage atlas = loadAtlas(m_source);
    if (atlas.isNull()) {
        return {};
    }
    const int perRow = std::max(1, atlas.width() / m_frameWidth);
    const int fx = (m_frame % perRow) * m_frameWidth;
    const int fy = (m_frame / perRow) * m_frameHeight;
    if (m_frame < 0 || fy + m_frameHeight > atlas.height()) {
        return {};
    }
    QList<QRgb> lut;
    for (const QVariant &v : m_colors) {
        const QColor c = v.value<QColor>();
        lut.append(c.isValid() ? qPremultiply(c.rgba()) : 0);
    }
    QImage out(m_frameWidth, m_frameHeight, QImage::Format_ARGB32_Premultiplied);
    out.fill(Qt::transparent);
    for (int y = 0; y < m_frameHeight; ++y) {
        auto *line = reinterpret_cast<QRgb *>(out.scanLine(y));
        for (int x = 0; x < m_frameWidth; ++x) {
            const int sx = fx + (m_mirror ? m_frameWidth - 1 - x : x);
            const int index = atlas.pixelIndex(sx, fy + y);
            if (index > 0 && index < lut.size()) {
                line[x] = lut.at(index);
            }
        }
    }
    return out;
}
