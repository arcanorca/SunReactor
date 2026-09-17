//! Shared visual grammar for every SunReactor workspace.
//!
//! One vocabulary keeps the screens coherent:
//!
//! * rounded panels own a functional region; the focused panel's border and
//!   title take the accent colour,
//! * `❯` marks the keyboard cursor, `›` the retained selection of an unfocused
//!   list,
//! * editable values show a capsule when focused (`‹ ›` when ←/→ adjust them),
//!   and read-only values are muted and never focusable,
//! * `●`/`○`/`▲` report state and `⎿` attaches a secondary detail line.
//!
//! Colour always reinforces a glyph or border change; it is never the only
//! signal.

use std::time::Instant;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        block::{Position, Title},
        Block, BorderType, Borders, Padding,
    },
};

use crate::tui::theme::SemanticStyles;

pub(crate) const CURSOR: &str = "❯ ";
pub(crate) const RETAINED: &str = "› ";
pub(crate) const BLANK_CURSOR: &str = "  ";
pub(crate) const DETAIL: &str = "⎿ ";
pub(crate) const DOT: &str = "●";
pub(crate) const HOLLOW: &str = "○";
pub(crate) const WARN: &str = "▲";
pub(crate) const BRAND_MARK: &str = "✻";
pub(crate) const SEPARATOR: &str = " · ";

/// A rounded region frame. Focus changes both the border and the title.
pub(crate) fn panel<'a>(title: &str, focused: bool, styles: &SemanticStyles) -> Block<'a> {
    let (border, title_style) = if focused {
        (styles.border_focused, styles.text_heading)
    } else {
        (
            styles.border_normal,
            styles.text_primary.add_modifier(Modifier::BOLD),
        )
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title.to_owned(), title_style),
            Span::raw(" "),
        ]))
        .padding(Padding::horizontal(1))
}

/// Adds right-aligned metadata to a panel's top border.
pub(crate) fn with_meta<'a>(block: Block<'a>, meta: Vec<Span<'a>>) -> Block<'a> {
    if meta.is_empty() {
        return block;
    }
    let mut spans = vec![Span::raw(" ")];
    spans.extend(meta);
    spans.push(Span::raw(" "));
    block.title(
        Title::from(Line::from(spans))
            .alignment(Alignment::Right)
            .position(Position::Top),
    )
}

/// A section heading inside a panel: bold text followed by a faint rule.
pub(crate) fn heading(text: &str, width: u16, styles: &SemanticStyles) -> Line<'static> {
    let used = Span::raw(text).width() + 1;
    let rule = usize::from(width).saturating_sub(used);
    Line::from(vec![
        Span::styled(
            text.to_owned(),
            styles.text_primary.add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled("─".repeat(rule), styles.border_normal),
    ])
}

/// How a field value may be changed from the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FieldKind {
    /// Information only; never focusable.
    ReadOnly,
    /// ←/→ step or cycle the value; Enter may also edit it.
    Adjustable,
    /// Enter opens a text editor.
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct FieldState {
    pub focused: bool,
    pub editing: bool,
}

impl FieldState {
    pub(crate) const fn new(focused: bool, editing: bool) -> Self {
        Self { focused, editing }
    }
}

/// Width in terminal cells, honouring wide Unicode characters.
pub(crate) fn cell_width(value: &str) -> usize {
    Span::raw(value).width()
}

/// Pads to a display width without splitting wide characters.
pub(crate) fn pad_to(value: &str, width: usize) -> String {
    let current = cell_width(value);
    if current >= width {
        return super::truncate(value, width);
    }
    format!("{value}{}", " ".repeat(width - current))
}

/// Right-aligns to a display width.
pub(crate) fn pad_left(value: &str, width: usize) -> String {
    let current = cell_width(value);
    if current >= width {
        return value.to_owned();
    }
    format!("{}{value}", " ".repeat(width - current))
}

/// The cursor gutter shared by every focusable row.
pub(crate) fn cursor_span(focused: bool, styles: &SemanticStyles) -> Span<'static> {
    if focused {
        Span::styled(CURSOR, styles.focus_marker)
    } else {
        Span::raw(BLANK_CURSOR)
    }
}

/// One label/value row. `edit_value` is shown instead of `value` while editing.
pub(crate) fn field_line(
    label: &str,
    label_width: usize,
    value: &str,
    kind: FieldKind,
    state: FieldState,
    edit_value: Option<&str>,
    styles: &SemanticStyles,
) -> Line<'static> {
    let focused = state.focused && kind != FieldKind::ReadOnly;
    let label_style = if kind == FieldKind::ReadOnly {
        styles.data_label
    } else if focused {
        styles.text_primary.add_modifier(Modifier::BOLD)
    } else {
        styles.text_primary
    };
    let mut spans = vec![
        cursor_span(focused, styles),
        Span::styled(pad_to(label, label_width), label_style),
        Span::raw(" "),
    ];
    spans.extend(value_spans(value, kind, state, edit_value, styles));
    Line::from(spans)
}

/// The value part of a field, usable in custom rows.
pub(crate) fn value_spans(
    value: &str,
    kind: FieldKind,
    state: FieldState,
    edit_value: Option<&str>,
    styles: &SemanticStyles,
) -> Vec<Span<'static>> {
    match kind {
        // Read-only values share the value column with editable ones.
        FieldKind::ReadOnly => vec![
            Span::raw("   "),
            Span::styled(value.to_owned(), styles.text_muted),
        ],
        // The terminal cursor is placed inside the capsule by the caller.
        _ if state.editing => vec![
            Span::styled("│", styles.focus_marker),
            Span::styled(
                format!(" {} ", edit_value.unwrap_or(value)),
                styles.value_capsule_editing,
            ),
            Span::styled("│", styles.focus_marker),
        ],
        FieldKind::Adjustable if state.focused => vec![
            Span::styled("‹ ", styles.focus_marker),
            Span::styled(format!(" {value} "), styles.value_capsule_focused),
            Span::styled(" ›", styles.focus_marker),
        ],
        FieldKind::Text if state.focused => vec![
            Span::raw("  "),
            Span::styled(format!(" {value} "), styles.value_capsule_focused),
        ],
        // Unfocused editable values stay aligned with focused capsules.
        _ => vec![
            Span::raw("   "),
            Span::styled(value.to_owned(), styles.text_primary),
        ],
    }
}

/// Column offset, from the start of the value, of the text cursor inside an
/// editing capsule (`│` plus one space).
pub(crate) const EDIT_CURSOR_PREFIX: u16 = 2;

/// Places the terminal cursor for an editing field rendered by [`field_line`].
pub(crate) fn place_field_cursor(
    f: &mut ratatui::Frame,
    row: Rect,
    label_width: usize,
    visual_cursor: usize,
) {
    let x = row.x + 2 + label_width as u16 + 1 + EDIT_CURSOR_PREFIX + visual_cursor as u16;
    if x < row.x + row.width {
        f.set_cursor(x, row.y);
    }
}

/// A read-only data row: muted label, primary value.
pub(crate) fn data_line(
    label: &str,
    label_width: usize,
    value: Vec<Span<'static>>,
    styles: &SemanticStyles,
) -> Line<'static> {
    let mut spans = vec![
        Span::raw(BLANK_CURSOR),
        Span::styled(pad_to(label, label_width), styles.data_label),
        Span::raw("    "),
    ];
    spans.extend(value);
    Line::from(spans)
}

/// A secondary detail attached to the row above.
pub(crate) fn detail_line(text: Vec<Span<'static>>, styles: &SemanticStyles) -> Line<'static> {
    let mut spans = vec![
        Span::raw(BLANK_CURSOR),
        Span::styled(DETAIL, styles.text_muted),
    ];
    spans.extend(text);
    Line::from(spans)
}

/// Claude-style activity glyphs for genuinely in-flight work.
const SPINNER_FRAMES: [&str; 6] = ["·", "✢", "✳", "✶", "✻", "✽"];

/// A ping-pong spinner frame derived from wall-clock time.
pub(crate) fn spinner_frame(started_at: Instant, now: Instant, animated: bool) -> &'static str {
    if !animated {
        return BRAND_MARK;
    }
    let step = (now.saturating_duration_since(started_at).as_millis() / 120) as usize;
    let cycle = SPINNER_FRAMES.len() * 2 - 2;
    let index = step % cycle;
    let index = if index < SPINNER_FRAMES.len() {
        index
    } else {
        cycle - index
    };
    SPINNER_FRAMES[index]
}

/// Text with a soft highlight band travelling across it, used only while a
/// request is actually pending.
pub(crate) fn shimmer_spans(
    text: &str,
    started_at: Instant,
    now: Instant,
    base: Style,
    highlight: Style,
    animated: bool,
) -> Vec<Span<'static>> {
    if !animated {
        return vec![Span::styled(text.to_owned(), base)];
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len().max(1);
    let period = (len + 8) as f64;
    let elapsed = now.saturating_duration_since(started_at).as_secs_f64();
    let head = (elapsed * 18.0) % period - 4.0;
    chars
        .into_iter()
        .enumerate()
        .map(|(index, character)| {
            let distance = (index as f64 - head).abs();
            let style = if distance < 2.5 { highlight } else { base };
            Span::styled(character.to_string(), style)
        })
        .collect()
}

/// Centres a `width` x `height` rectangle inside `area`.
pub(crate) fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}
