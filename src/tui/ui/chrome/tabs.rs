use std::time::Instant;

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::super::kit;
use crate::tui::theme::SemanticStyles;
use crate::tui::{Model, Tab};

const TAB_GAP: u16 = 3;

fn tab_label(tab: Tab, compact: bool) -> String {
    if compact {
        let short = match tab {
            Tab::Monitors => "Mon",
            Tab::Limits => "Auto",
            Tab::Location => "Loc",
            Tab::Weather => "Wx",
            Tab::Settings => "Set",
        };
        format!("{} {short}", tab.index() + 1)
    } else {
        format!("{} {}", tab.index() + 1, tab.title())
    }
}

/// Tabs sit on a hairline rule; a heavier accent segment under the active tab
/// makes the current workspace visible without brackets or colour alone.
pub(crate) fn render_tabs(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let compact = area.width < 72;
    let labels = Tab::ALL
        .iter()
        .map(|tab| tab_label(*tab, compact))
        .collect::<Vec<_>>();
    let mut positions = Vec::with_capacity(labels.len());
    let mut x = 1u16;
    // Focus on the tab bar shows the shared cursor and a capsule on the
    // active workspace, exactly like a focused field.
    let mut spans = vec![if app.tabs_focused {
        Span::styled("❯", styles.focus_marker)
    } else {
        Span::raw(" ")
    }];
    for (index, (tab, label)) in Tab::ALL.iter().zip(&labels).enumerate() {
        if index > 0 {
            spans.push(Span::raw(" ".repeat(usize::from(TAB_GAP))));
            x += TAB_GAP;
        }
        let width = kit::cell_width(label) as u16;
        positions.push((x, width));
        x += width;
        if *tab == app.active_tab {
            spans.push(Span::styled(
                label.clone(),
                if app.tabs_focused {
                    styles.value_capsule_focused
                } else {
                    styles.tab_active
                },
            ));
        } else {
            spans.push(Span::styled(label.clone(), styles.tab_inactive));
        }
    }
    f.render_widget(
        Paragraph::new(Line::from(spans)),
        Rect::new(area.x, area.y, area.width, 1),
    );

    if area.height < 2 {
        return;
    }
    let active = app.active_tab.index();
    let (start, width) = match app.motion.tab_slide_position(Instant::now()) {
        Some(position) => {
            let lower = position.floor().clamp(0.0, 4.0) as usize;
            let upper = (lower + 1).min(positions.len() - 1);
            let t = position - lower as f32;
            let lerp =
                |a: u16, b: u16| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u16;
            (
                lerp(positions[lower].0, positions[upper].0),
                lerp(positions[lower].1, positions[upper].1),
            )
        }
        None => positions[active],
    };
    let rule_width = usize::from(area.width);
    let start = usize::from(start).min(rule_width);
    let end = (start + usize::from(width)).min(rule_width);
    let rule = Line::from(vec![
        Span::styled("─".repeat(start), styles.border_normal),
        Span::styled("━".repeat(end - start), styles.focus_marker),
        Span::styled("─".repeat(rule_width - end), styles.border_normal),
    ]);
    f.render_widget(
        Paragraph::new(rule),
        Rect::new(area.x, area.y + 1, area.width, 1),
    );
}
