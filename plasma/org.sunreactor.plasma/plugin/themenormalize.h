#pragma once

#include <QJsonObject>
#include <QString>
#include <QStringList>
#include <QVariantMap>

/*!
 * Turns the daemon's theme JSON into the one shape QML consumes.
 *
 * The daemon resolves themes (built-in < user file, aliases, family defaults)
 * and sends `status.theme` / `get_theme` objects as in DESIGN.md section 3. This
 * code only validates and fills gaps, so a malformed or partial object can never
 * reach QML as half a theme:
 *
 *   { id, name, family, glyphStyle, legacy,
 *     palette: { bg, fg, accent, secondaryAccent, borderActive, borderInactive,
 *                textMuted, success, warning, error },          // all "#rrggbb"
 *     widget:  { renderer, font, pixelScale, shades, density, border, radius,
 *                dither, glow, modules } }
 *
 * - Every widget key the daemon omitted or sent out of range falls back to the
 *   family default (the table in DESIGN.md section 2).
 * - `shades` is passed through only when it holds 2..8 valid colours; otherwise
 *   it is empty and the QML Theme derives shades from the palette.
 * - An older daemon sends only the six flat legacy fields. The family is then
 *   looked up by the exact theme name in the 35-entry built-in table (no
 *   substring guessing), the missing palette entries are derived, and
 *   `legacy` is true. This is the only place the legacy table lives.
 * - Without a usable bg/fg pair the result is an empty map: QML then falls back
 *   to the Plasma colour scheme instead of inventing a theme.
 */
namespace SunReactor::ThemeData
{

inline const QStringList &families()
{
    static const QStringList v{QStringLiteral("pixel"), QStringLiteral("instrument"), QStringLiteral("terminal")};
    return v;
}

inline const QStringList &moduleIds()
{
    static const QStringList v{QStringLiteral("telemetry"), QStringLiteral("curve"), QStringLiteral("displays"),
                               QStringLiteral("automation"), QStringLiteral("weather"), QStringLiteral("forecast"),
                               QStringLiteral("suntimes")};
    return v;
}

/*! Widget block with every key at the family's default. Unknown family -> instrument. */
[[nodiscard]] QVariantMap familyDefaults(const QString &family);

/*! Full normalisation of a `status.theme` / `get_theme` object (see above). */
[[nodiscard]] QVariantMap normalizeTheme(const QJsonObject &theme);

/*! One `list_themes` entry: { id, name, family, bg, fg, accent, secondaryAccent }, or empty if unusable. */
[[nodiscard]] QVariantMap normalizeSummary(const QJsonObject &entry);

/*! Legacy name/id -> { id, family } from the built-in table; empty when unknown. */
[[nodiscard]] QVariantMap legacyLookup(const QString &nameOrId);

/*! "#RRGGBB"/"#rrggbb" -> "#rrggbb"; anything else -> empty string. */
[[nodiscard]] QString normalizeColor(const QString &value);

} // namespace SunReactor::ThemeData
