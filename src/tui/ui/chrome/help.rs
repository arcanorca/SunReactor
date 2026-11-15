use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::super::kit;
use crate::tui::theme::Palette;
use crate::tui::Model;

pub(crate) fn render_help(f: &mut Frame, app: &Model, palette: &Palette) {
    let screen = f.size();
    let styles = palette.styles();
    let width = screen.width.saturating_sub(4).clamp(30, 72);
    let height = screen.height.saturating_sub(2).clamp(8, 26);
    let area = kit::centered(screen, width, height);

    f.render_widget(Clear, area);

    let context = crate::tui::command::commands_for_workspace(app);
    let key_width = 16;
    let inner_width = area.width.saturating_sub(4);
    let mut lines = Vec::new();

    let command_line = |keys: &'static str, action: &'static str| {
        Line::from(vec![
            Span::styled(
                kit::pad_to(keys, key_width),
                styles.key_hint.add_modifier(Modifier::BOLD),
            ),
            Span::styled(action, styles.text_primary),
        ])
    };

    lines.push(kit::heading(context.context.title(), inner_width, &styles));
    if context.current_commands.is_empty() {
        lines.push(Line::from(Span::styled(
            "Nothing to adjust here.",
            styles.text_muted,
        )));
    } else {
        for command in context.current_commands {
            lines.push(command_line(command.keys, command.action));
        }
    }

    if !context.global_commands.is_empty() {
        lines.push(Line::from(""));
        lines.push(kit::heading("Everywhere", inner_width, &styles));
        for command in context.global_commands {
            lines.push(command_line(command.keys, command.action));
        }
    }

    lines.push(Line::from(""));
    lines.push(kit::heading("Symbols", inner_width, &styles));
    for item in crate::tui::command::OFFICIAL_SYMBOLS {
        lines.push(Line::from(vec![
            Span::styled(kit::pad_to(item.symbol, key_width), styles.focus_marker),
            Span::styled(item.meaning, styles.text_muted),
        ]));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("Effects  ", styles.data_label),
        Span::styled(app.config.tui.effects.label(), styles.text_primary),
    ]));

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = lines.len().saturating_sub(usize::from(inner_height));
    let scroll = app.help_scroll.min(max_scroll);
    let title = if max_scroll > 0 {
        format!("Help · {}/{}", scroll + 1, max_scroll + 1)
    } else {
        String::from("Help")
    };
    let block = kit::with_meta(
        kit::panel(&title, true, &styles),
        vec![
            Span::styled("Esc", styles.key_hint),
            Span::styled(" close", styles.key_hint_desc),
        ],
    );

    f.render_widget(
        Paragraph::new(lines)
            .scroll((scroll as u16, 0))
            .style(styles.base)
            .block(block),
        area,
    );
}
