//! Small display fonts shared by the automation and weather workspaces.

/// Three-row rounded glyphs for large numbers: digits and a minus sign.
const ROUNDED_DIGITS: [[&str; 3]; 10] = [
    ["╭─╮", "│ │", "╰─╯"],
    ["╶╮ ", " │ ", "╶┴╴"],
    ["╭─╮", "╭─╯", "╰─╴"],
    ["╶─╮", " ─┤", "╶─╯"],
    ["╷ ╷", "╰─┤", "  ╵"],
    ["╭─╴", "╰─╮", "╶─╯"],
    ["╭─╴", "├─╮", "╰─╯"],
    ["╶─╮", "  │", "  ╵"],
    ["╭─╮", "├─┤", "╰─╯"],
    ["╭─╮", "╰─┤", "╶─╯"],
];

/// Renders `text` in the three-row rounded number font, with one cell between
/// glyphs. Characters other than digits and `-` are skipped.
pub(crate) fn rounded_number_rows(text: &str) -> [String; 3] {
    let mut rows: [String; 3] = Default::default();
    let glyphs = text.chars().filter_map(|character| match character {
        '-' => Some(["   ", "╶─╴", "   "]),
        digit => digit
            .to_digit(10)
            .map(|digit| ROUNDED_DIGITS[digit as usize]),
    });
    for (index, glyph) in glyphs.enumerate() {
        for (row, line) in rows.iter_mut().enumerate() {
            if index > 0 {
                line.push(' ');
            }
            line.push_str(glyph[row]);
        }
    }
    rows
}

/// Three-row pixel art block glyphs for weather temperature displays.
const WEATHER_PIXEL_DIGITS: [[&str; 3]; 10] = [
    ["▄▀▀▄", "█  █", " ▀▀ "], // 0
    [" ▄█ ", "  █ ", " ▀▀▀"], // 1
    ["█▀▀▄", " ▄▀ ", "▀▀▀▀"], // 2
    ["▀▀▀█", " ▀▀█", " ▀▀ "], // 3
    ["█  █", "▀▀▀█", "   ▀"], // 4
    ["█▀▀▀", "▀▀▀▄", " ▀▀ "], // 5
    ["▄▀▀▀", "█▀▀▄", " ▀▀ "], // 6
    ["▀▀▀█", "  █ ", "  ▀ "], // 7
    ["▄▀▀▄", "█▀▀█", " ▀▀ "], // 8
    ["▄▀▀▄", " ▀▀█", " ▀▀ "], // 9
];

/// Renders `text` in the three-row weather pixel block font, with one cell between
/// glyphs. Characters other than digits and `-` are skipped.
pub(crate) fn weather_pixel_font_rows(text: &str) -> [String; 3] {
    let mut rows: [String; 3] = Default::default();
    let glyphs = text.chars().filter_map(|character| match character {
        '-' => Some(["    ", "▀▀▀▀", "    "]),
        digit => digit
            .to_digit(10)
            .map(|digit| WEATHER_PIXEL_DIGITS[digit as usize]),
    });
    for (index, glyph) in glyphs.enumerate() {
        for (row, line) in rows.iter_mut().enumerate() {
            if index > 0 {
                line.push(' ');
            }
            line.push_str(glyph[row]);
        }
    }
    rows
}

/// A 5x7 dot-matrix font, like an LED display.
#[allow(dead_code)]
const DOT_MATRIX_GLYPHS: [(char, [&str; 7]); 11] = [
    (
        '0',
        [
            ".###.", "#...#", "#..##", "#.#.#", "##..#", "#...#", ".###.",
        ],
    ),
    (
        '1',
        [
            "..#..", ".##..", "..#..", "..#..", "..#..", "..#..", ".###.",
        ],
    ),
    (
        '2',
        [
            ".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#####",
        ],
    ),
    (
        '3',
        [
            "#####", "...#.", "..#..", "...#.", "....#", "#...#", ".###.",
        ],
    ),
    (
        '4',
        [
            "...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.",
        ],
    ),
    (
        '5',
        [
            "#####", "#....", "####.", "....#", "....#", "#...#", ".###.",
        ],
    ),
    (
        '6',
        [
            "..##.", ".#...", "#....", "####.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '7',
        [
            "#####", "....#", "...#.", "..#..", ".#...", ".#...", ".#...",
        ],
    ),
    (
        '8',
        [
            ".###.", "#...#", "#...#", ".###.", "#...#", "#...#", ".###.",
        ],
    ),
    (
        '9',
        [
            ".###.", "#...#", "#...#", ".####", "....#", "...#.", ".##..",
        ],
    ),
    (
        '-',
        [
            ".....", ".....", ".....", "#####", ".....", ".....", ".....",
        ],
    ),
];

/// Renders `text` as four Braille dot-matrix rows. Every font dot is one
/// Braille dot, two dot positions apart in both directions.
#[allow(dead_code)]
pub(crate) fn braille_dot_matrix_rows(text: &str) -> [String; 4] {
    let glyphs: Vec<&[&str; 7]> = text
        .chars()
        .filter_map(|c| DOT_MATRIX_GLYPHS.iter().find(|(key, _)| *key == c))
        .map(|(_, rows)| rows)
        .collect();
    let mut rows: [String; 4] = Default::default();
    for (cell_row, out) in rows.iter_mut().enumerate() {
        for (index, glyph) in glyphs.iter().enumerate() {
            if index > 0 {
                out.push(' ');
            }
            for column in 0..5 {
                let lit = |font_row: usize| {
                    glyph
                        .get(font_row)
                        .is_some_and(|row| row.as_bytes()[column] == b'#')
                };
                // Font rows 2n and 2n+1 share a cell, on Braille rows 0 and 2.
                let mut bits = 0_u32;
                if lit(cell_row * 2) {
                    bits |= 0x01;
                }
                if lit(cell_row * 2 + 1) {
                    bits |= 0x04;
                }
                out.push(char::from_u32(0x2800 + bits).unwrap_or(' '));
            }
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{braille_dot_matrix_rows, rounded_number_rows, weather_pixel_font_rows};

    #[test]
    fn rounded_number_font_skips_unsupported_characters() {
        assert_eq!(
            rounded_number_rows("1x-"),
            ["╶╮     ", " │  ╶─╴", "╶┴╴    "]
        );
    }

    #[test]
    fn weather_pixel_font_skips_unsupported_characters() {
        assert_eq!(
            weather_pixel_font_rows("1x-"),
            [" ▄█      ", "  █  ▀▀▀▀", " ▀▀▀     "]
        );
    }

    #[test]
    fn weather_pixel_font_preserves_digit_geometry() {
        let single = weather_pixel_font_rows("7");
        assert_eq!(single, ["▀▀▀█", "  █ ", "  ▀ "]);
        let pair = weather_pixel_font_rows("23");
        assert!(pair.iter().all(|row| row.chars().count() == 9));
        let negative = weather_pixel_font_rows("-4");
        assert_eq!(negative[1], "▀▀▀▀ ▀▀▀█");
    }

    #[test]
    fn braille_dot_matrix_font_preserves_digit_geometry() {
        let rows = braille_dot_matrix_rows("1");
        // "..#.." over ".##..": the middle column has both dots in cell row 0.
        assert_eq!(rows[0], "⠀⠄⠅⠀⠀");
        // The last font row ".###." sits alone on Braille row 0 of cell row 3.
        assert_eq!(rows[3], "⠀⠁⠁⠁⠀");
        let pair = braille_dot_matrix_rows("23");
        assert!(pair.iter().all(|row| row.chars().count() == 11));
        let negative = braille_dot_matrix_rows("-4");
        assert_eq!(negative[1].chars().take(5).collect::<String>(), "⠄⠄⠄⠄⠄");
    }
}
