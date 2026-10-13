#pragma once

#include <QHash>
#include <QtGlobal>

#include <array>

/*
 * Glyph bitmaps shared by the pixel renderer (PixelGlyphs in pixelart.cpp) and
 * the dot-matrix renderer (DotText in dotmatrix.cpp), so both draw the same
 * letters from one table. A glyph is `rows` rows of `width` columns; bit
 * (width - 1 - x) of bits[y] is the cell at column x, row y. Rows past `rows`
 * are zero.
 */
namespace GlyphTables {

struct Glyph {
    int width = 0;
    int rows = 0;
    std::array<quint8, 9> bits{};
};

using Face = QHash<char32_t, Glyph>;

/*! 5x7 proportional: capitals, lower case (x-height 5), digits, punctuation,
 *  Turkish and German letters. Rows 7..8 hold descenders. */
const Face &smallFace();
/*! 3x5 capitals, digits and the few symbols axis labels need. */
const Face &tinyFace();
/*! Topaz-style 8x8 monospace face (Amiga Workbench system font): 8 columns per
 *  glyph, cap rows 0..6, descender row 7; the ellipsis, the triangle used by
 *  drop-down gadgets, degree sign and middle dot are included. */
const Face &topazFace();
/*! PETSCII-style 4x8 text-mode face (Commodore 64 screen, the C64 skin): one
 *  4-column cell per character so 36+ cells fit a 19 gu popup at integer art
 *  pixels; cap rows 0..6, row 7 blank except descenders and graphics; the
 *  box-drawing, half/eighth/full blocks, shade blocks, the bullets, the
 *  triangles, sun/cloud symbols and the ellipsis / middle dot / degree sign. */
const Face &petsciiFace();
/*! Seven-segment cells: bit 0 a (top) .. bit 6 g (middle). */
const QHash<char32_t, quint8> &segFace();
/*! Narrow seven-segment cells (colon and full stop). */
bool isSegNarrow(char32_t c);

} // namespace GlyphTables
