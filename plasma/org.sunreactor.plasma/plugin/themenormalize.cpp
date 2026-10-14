#include "themenormalize.h"

#include <QColor>
#include <QHash>
#include <QJsonArray>
#include <QJsonValue>
#include <QRegularExpression>
#include <QVariantList>

#include <algorithm>
#include <cmath>

namespace SunReactor::ThemeData
{

namespace
{

struct LegacyEntry {
    const char *id;
    const char *family;
};

/// Built-in themes as an older daemon names them (`Theme::name()`), keyed by
/// the name folded to [a-z0-9]. Ids match the registry ids in DESIGN.md.
const QHash<QString, LegacyEntry> &legacyTable()
{
    static const QHash<QString, LegacyEntry> table = [] {
        QHash<QString, LegacyEntry> t;
        const auto add = [&t](const char *key, const char *id, const char *family) {
            t.insert(QString::fromLatin1(key), LegacyEntry{id, family});
        };
        // pixel
        add("handheld", "handheld", "pixel");
        add("commodore64", "commodore64", "pixel");
        add("amiga", "amiga", "pixel");
        add("classicmacintosh", "classicmacintosh", "pixel");
        add("casiodigital", "casiodigital", "pixel");
        add("nothing", "nothing", "pixel");
        add("cyberpunk", "cyberpunk", "pixel");
        add("synthwave84", "synthwave84", "pixel");
        add("hackergreen", "hackergreen", "pixel");
        // instrument
        add("braun", "braun", "instrument");
        add("vfdhifi", "vfd_hifi", "instrument");
        add("nixie", "nixie", "instrument");
        add("oscilloscope", "oscilloscope", "instrument");
        add("thinkpad", "thinkpad", "instrument");
        add("unixworkstation", "unixworkstation", "instrument");
        add("phosphorblue", "phosphorblue", "instrument");
        add("amber", "amber", "instrument");
        add("grayscale", "grayscale", "instrument");
        // terminal
        add("dracula", "dracula", "terminal");
        add("nord", "nord", "terminal");
        add("gruvbox", "gruvbox", "terminal");
        add("catppuccinmocha", "catppuccinmocha", "terminal");
        add("tokyonight", "tokyonight", "terminal");
        add("onedark", "onedark", "terminal");
        add("ayudark", "ayudark", "terminal");
        add("ayumirage", "ayumirage", "terminal");
        add("solarizeddark", "solarizeddark", "terminal");
        add("everforest", "everforest", "terminal");
        add("kanagawa", "kanagawa", "terminal");
        add("zenburn", "zenburn", "terminal");
        add("monokai", "monokai", "terminal");
        add("nightowl", "nightowl", "terminal");
        add("materialocean", "materialocean", "terminal");
        add("rosepine", "rosepine", "terminal");
        add("terminal", "terminal", "terminal");
        return t;
    }();
    return table;
}

QString foldKey(const QString &value)
{
    const QString decomposed = value.normalized(QString::NormalizationForm_KD).toLower();
    QString key;
    key.reserve(decomposed.size());
    for (const QChar c : decomposed) {
        if ((c >= QLatin1Char('a') && c <= QLatin1Char('z')) || (c >= QLatin1Char('0') && c <= QLatin1Char('9'))) {
            key.append(c);
        }
    }
    return key;
}

QString oneOf(const QJsonValue &value, std::initializer_list<const char *> allowed, const QString &fallback)
{
    const QString s = value.toString();
    for (const char *a : allowed) {
        if (s == QLatin1String(a)) {
            return s;
        }
    }
    return fallback;
}

QString mix(const QString &a, const QString &b, double t)
{
    const QColor ca(a);
    const QColor cb(b);
    const auto lerp = [t](int x, int y) { return static_cast<int>(std::lround(x + (y - x) * t)); };
    return QColor(lerp(ca.red(), cb.red()), lerp(ca.green(), cb.green()), lerp(ca.blue(), cb.blue()))
        .name(QColor::HexRgb);
}

const QStringList &defaultModules()
{
    return moduleIds();
}

} // namespace

QString normalizeColor(const QString &value)
{
    static const QRegularExpression hex(QStringLiteral("^#[0-9a-fA-F]{6}$"));
    const QString trimmed = value.trimmed();
    return hex.match(trimmed).hasMatch() ? trimmed.toLower() : QString();
}

QVariantMap familyDefaults(const QString &family)
{
    QVariantMap w;
    w[QStringLiteral("pixelScale")] = 0;
    w[QStringLiteral("shades")] = QVariantList{};
    w[QStringLiteral("glow")] = 0.0;
    w[QStringLiteral("modules")] = QVariant(defaultModules());
    if (family == QLatin1String("pixel")) {
        w[QStringLiteral("renderer")] = QStringLiteral("pixel");
        w[QStringLiteral("font")] = QStringLiteral("bitmap8");
        w[QStringLiteral("density")] = QStringLiteral("compact");
        w[QStringLiteral("border")] = QStringLiteral("line");
        w[QStringLiteral("radius")] = 0;
        w[QStringLiteral("dither")] = QStringLiteral("bayer4");
    } else if (family == QLatin1String("terminal")) {
        w[QStringLiteral("renderer")] = QStringLiteral("terminal");
        w[QStringLiteral("font")] = QStringLiteral("mono");
        w[QStringLiteral("density")] = QStringLiteral("compact");
        w[QStringLiteral("border")] = QStringLiteral("line");
        w[QStringLiteral("radius")] = 0;
        w[QStringLiteral("dither")] = QStringLiteral("none");
    } else {
        w[QStringLiteral("renderer")] = QStringLiteral("vector");
        w[QStringLiteral("font")] = QStringLiteral("sans");
        w[QStringLiteral("density")] = QStringLiteral("comfortable");
        w[QStringLiteral("border")] = QStringLiteral("line");
        w[QStringLiteral("radius")] = 4;
        w[QStringLiteral("dither")] = QStringLiteral("none");
    }
    return w;
}

QVariantMap legacyLookup(const QString &nameOrId)
{
    const auto it = legacyTable().constFind(foldKey(nameOrId));
    if (it == legacyTable().constEnd()) {
        return {};
    }
    return {{QStringLiteral("id"), QString::fromLatin1(it->id)},
            {QStringLiteral("family"), QString::fromLatin1(it->family)}};
}

QVariantMap normalizeTheme(const QJsonObject &theme)
{
    const QJsonObject palette = theme.value(QStringLiteral("palette")).toObject();
    const bool legacy = !theme.value(QStringLiteral("palette")).isObject();

    // Palette entries come from `palette`, or from the flat legacy fields.
    const auto colour = [&](const char *key) {
        const QString fromPalette = normalizeColor(palette.value(QLatin1String(key)).toString());
        return fromPalette.isEmpty() ? normalizeColor(theme.value(QLatin1String(key)).toString()) : fromPalette;
    };

    const QString bg = colour("bg");
    const QString fg = colour("fg");
    if (bg.isEmpty() || fg.isEmpty()) {
        return {};
    }
    const QString accent = colour("accent").isEmpty() ? fg : colour("accent");
    const QString secondary = colour("secondary_accent").isEmpty() ? accent : colour("secondary_accent");
    const QString muted = colour("text_muted").isEmpty() ? mix(bg, fg, 0.6) : colour("text_muted");
    const auto orDerived = [&](const char *key, const QString &derived) {
        const QString c = colour(key);
        return c.isEmpty() ? derived : c;
    };

    QVariantMap pal;
    pal[QStringLiteral("bg")] = bg;
    pal[QStringLiteral("fg")] = fg;
    pal[QStringLiteral("accent")] = accent;
    pal[QStringLiteral("secondaryAccent")] = secondary;
    pal[QStringLiteral("textMuted")] = muted;
    pal[QStringLiteral("borderActive")] = orDerived("border_active", accent);
    pal[QStringLiteral("borderInactive")] = orDerived("border_inactive", mix(bg, muted, 0.5));
    pal[QStringLiteral("success")] = orDerived("success", secondary);
    pal[QStringLiteral("warning")] = orDerived("warning", accent);
    pal[QStringLiteral("error")] = orDerived("error", QStringLiteral("#d9534f"));

    const QString name = theme.value(QStringLiteral("name")).toString().trimmed();
    QString id = theme.value(QStringLiteral("id")).toString().trimmed();
    QString family = theme.value(QStringLiteral("family")).toString();
    if (!families().contains(family) || id.isEmpty()) {
        const QVariantMap known = legacyLookup(id.isEmpty() ? name : id);
        if (id.isEmpty()) {
            id = known.value(QStringLiteral("id")).toString();
        }
        if (!families().contains(family)) {
            family = known.value(QStringLiteral("family"), QStringLiteral("instrument")).toString();
        }
    }

    const QVariantMap defaults = familyDefaults(family);
    const QJsonObject widget = theme.value(QStringLiteral("widget")).toObject();
    QVariantMap w = defaults;

    w[QStringLiteral("renderer")] = oneOf(widget.value(QStringLiteral("renderer")), {"pixel", "vector", "terminal"},
                                          defaults.value(QStringLiteral("renderer")).toString());
    w[QStringLiteral("font")] = oneOf(widget.value(QStringLiteral("font")),
                                      {"bitmap8", "bitmap_vga", "mono", "sans", "display"},
                                      defaults.value(QStringLiteral("font")).toString());
    w[QStringLiteral("density")] = oneOf(widget.value(QStringLiteral("density")), {"compact", "comfortable"},
                                         defaults.value(QStringLiteral("density")).toString());
    w[QStringLiteral("border")] = oneOf(widget.value(QStringLiteral("border")),
                                        {"none", "line", "bevel", "inset", "double"},
                                        defaults.value(QStringLiteral("border")).toString());
    w[QStringLiteral("dither")] = oneOf(widget.value(QStringLiteral("dither")), {"none", "bayer2", "bayer4"},
                                        defaults.value(QStringLiteral("dither")).toString());

    const QJsonValue scale = widget.value(QStringLiteral("pixel_scale"));
    if (scale.isDouble() && scale.toInt(-1) >= 0 && scale.toInt(-1) <= 6) {
        w[QStringLiteral("pixelScale")] = scale.toInt();
    }
    const QJsonValue radius = widget.value(QStringLiteral("radius"));
    if (radius.isDouble()) {
        w[QStringLiteral("radius")] = qBound(0, radius.toInt(), 24);
    }
    const QJsonValue glow = widget.value(QStringLiteral("glow"));
    if (glow.isDouble()) {
        w[QStringLiteral("glow")] = qBound(0.0, glow.toDouble(), 1.0);
    }

    QVariantList shades;
    const QJsonArray shadeArray = widget.value(QStringLiteral("shades")).toArray();
    for (const QJsonValue &v : shadeArray) {
        const QString c = normalizeColor(v.toString());
        if (c.isEmpty()) {
            shades.clear();
            break;
        }
        shades.append(c);
    }
    if (shades.size() < 2 || shades.size() > 8) {
        shades.clear();
    }
    w[QStringLiteral("shades")] = shades;

    QStringList modules;
    const QJsonArray moduleArray = widget.value(QStringLiteral("modules")).toArray();
    for (const QJsonValue &v : moduleArray) {
        const QString m = v.toString();
        if (moduleIds().contains(m) && !modules.contains(m)) {
            modules.append(m);
        }
    }
    // An absent list means "the default set"; a list the daemon sent is
    // respected even when it hides modules, as long as something is left.
    w[QStringLiteral("modules")] = QVariant(modules.isEmpty() ? defaultModules() : modules);

    QVariantMap out;
    out[QStringLiteral("id")] = id;
    out[QStringLiteral("name")] = name.isEmpty() ? id : name;
    out[QStringLiteral("family")] = family;
    out[QStringLiteral("glyphStyle")] =
        oneOf(theme.value(QStringLiteral("glyph_style")), {"rounded", "pixel"},
              family == QLatin1String("pixel") ? QStringLiteral("pixel") : QStringLiteral("rounded"));
    out[QStringLiteral("legacy")] = legacy;
    out[QStringLiteral("palette")] = pal;
    out[QStringLiteral("widget")] = w;
    return out;
}

QVariantMap normalizeSummary(const QJsonObject &entry)
{
    const QString id = entry.value(QStringLiteral("id")).toString().trimmed();
    const QString bg = normalizeColor(entry.value(QStringLiteral("bg")).toString());
    const QString fg = normalizeColor(entry.value(QStringLiteral("fg")).toString());
    if (id.isEmpty() || bg.isEmpty() || fg.isEmpty()) {
        return {};
    }
    const QString accent = normalizeColor(entry.value(QStringLiteral("accent")).toString());
    const QString secondary = normalizeColor(entry.value(QStringLiteral("secondary_accent")).toString());
    QString family = entry.value(QStringLiteral("family")).toString();
    if (!families().contains(family)) {
        family = legacyLookup(id).value(QStringLiteral("family"), QStringLiteral("instrument")).toString();
    }
    const QString name = entry.value(QStringLiteral("name")).toString().trimmed();
    return {
        {QStringLiteral("id"), id},
        {QStringLiteral("name"), name.isEmpty() ? id : name},
        {QStringLiteral("family"), family},
        {QStringLiteral("bg"), bg},
        {QStringLiteral("fg"), fg},
        {QStringLiteral("accent"), accent.isEmpty() ? fg : accent},
        {QStringLiteral("secondaryAccent"), secondary.isEmpty() ? (accent.isEmpty() ? fg : accent) : secondary},
    };
}

} // namespace SunReactor::ThemeData
