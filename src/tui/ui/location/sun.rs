use ratatui::{
    layout::Rect,
    text::Span,
    widgets::{Clear, Paragraph},
    Frame,
};

use crate::policy::AutomationMilestone;
use crate::tui::ui::kit;
use crate::tui::{theme::SemanticStyles, Model};

use super::LABEL_WIDTH;

/// Read-only solar facts for the configured location, all taken from the
/// daemon status or the shared milestone schedule.
pub(super) fn render_sun_panel(f: &mut Frame, app: &Model, area: Rect, styles: &SemanticStyles) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let block = kit::panel("Sun today", false, styles);
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block.style(styles.base), area);

    let use_12h = app.config.tui.use_12h_time;
    let format = |time: chrono::DateTime<chrono::FixedOffset>| {
        if use_12h {
            time.format("%I:%M %p").to_string()
        } else {
            time.format("%H:%M").to_string()
        }
    };
    let status = app.status.as_ref();
    let sunrise = status
        .and_then(|status| status.sunrise_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let sunset = status
        .and_then(|status| status.sunset_epoch_s)
        .and_then(|epoch| app.local_time_at_epoch(epoch));
    let noon = app
        .selected_monitor_schedule()
        .or_else(|| app.monitor_milestones.first())
        .and_then(|schedule| {
            schedule
                .milestones
                .iter()
                .find(|milestone| milestone.milestone == AutomationMilestone::Peak)
        })
        .map(|milestone| milestone.base_time_local);
    let dash = || vec![Span::styled("—", styles.text_muted)];
    let value = |text: String| vec![Span::styled(text, styles.text_primary)];

    let mut lines = vec![
        kit::data_line(
            "Sunrise",
            LABEL_WIDTH + 1,
            sunrise.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
        kit::data_line(
            "Solar noon",
            LABEL_WIDTH + 1,
            noon.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
        kit::data_line(
            "Sunset",
            LABEL_WIDTH + 1,
            sunset.map_or_else(dash, |t| value(format(t))),
            styles,
        ),
    ];
    if let (Some(rise), Some(set)) = (sunrise, sunset) {
        let minutes = (set - rise).num_minutes().max(0);
        lines.push(kit::data_line(
            "Daylight",
            LABEL_WIDTH + 1,
            value(format!("{}h {:02}m", minutes / 60, minutes % 60)),
            styles,
        ));
    }
    if let Some(elevation) = status.and_then(|status| status.solar_elevation) {
        lines.push(kit::data_line(
            "Sun now",
            LABEL_WIDTH + 1,
            value(format!("{elevation:+.1}°")),
            styles,
        ));
    }
    f.render_widget(Paragraph::new(lines), inner);
}
