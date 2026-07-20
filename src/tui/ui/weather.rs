use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, List, ListItem, Paragraph},
    Frame,
};

use crate::tui::Model;

use super::settings::render_settings_layout;
use super::weather_model::{weather_panel_state, WeatherPanelData, WeatherPanelState};
use crate::tui::theme::Palette;

pub(super) fn render_weather(f: &mut Frame, app: &Model, area: Rect) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)])
        .split(area);

    render_weather_config(f, app, sections[1]);
    render_weather_panel(f, app, sections[0]);
}

fn render_weather_config(f: &mut Frame, app: &Model, area: Rect) {
    render_settings_layout(
        f,
        app,
        area,
        " Weather Config ",
        vec![(
            String::from("OpenWeather API Key"),
            app.form.api_key_input.value().to_string(),
            0,
        )],
        &app.config.tui.theme.palette(),
    );
}

fn render_weather_panel(f: &mut Frame, app: &Model, area: Rect) {
    let palette = app.config.tui.theme.palette();
    
    let state = weather_panel_state(
        app.status.as_ref(),
        app.config.tui.use_12h_time,
        &app.config.location.timezone,
        app.config.tui.temperature_unit,
        &palette,
    );

    let title = match state {
        WeatherPanelState::Message(_) => " Weather Status ",
        WeatherPanelState::Ready(_) => " Sky Condition & 24h Forecast ",
    };

    let weather_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.border_inactive))
        .title(Span::styled(
            title,
            Style::default()
                .fg(palette.fg)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = weather_block.inner(area);
    f.render_widget(weather_block, area);

    let weather_layout = padded_weather_layout(inner);
    match state {
        WeatherPanelState::Message(message) => render_weather_message(f, inner, &message, &palette),
        WeatherPanelState::Ready(panel) => {
            render_weather_header(f, &panel, weather_layout[0], &palette);
            render_weather_details(f, &panel, weather_layout[1], &palette);
        }
    }
}

fn padded_weather_layout(area: Rect) -> Vec<Rect> {
    let content_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area)[1];
    let horizontal_padded = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(content_area)[1];

    Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(horizontal_padded)
        .to_vec()
}

fn render_weather_message(f: &mut Frame, area: Rect, message: &str, palette: &Palette) {
    let paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .wrap(ratatui::widgets::Wrap { trim: true })
        .style(
            Style::default()
                .fg(palette.text_muted)
                .add_modifier(Modifier::DIM),
        );
    f.render_widget(paragraph, area);
}

fn render_weather_header(f: &mut Frame, panel: &WeatherPanelData, area: Rect, palette: &Palette) {
    let header = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(16), Constraint::Min(0)])
        .split(area);

    let art_paragraph = Paragraph::new(panel.header.art)
        .alignment(Alignment::Left)
        .style(Style::default().fg(panel.header.art_color));
    let text_paragraph = Paragraph::new(format!(
        "\n  {}\n  {}\n  {}\n  {}",
        panel.header.sunrise_label,
        panel.header.sunset_label,
        panel.header.temperature_label,
        panel.header.cloud_label
    ))
    .alignment(Alignment::Left)
    .style(Style::default().fg(palette.fg).add_modifier(Modifier::BOLD));

    f.render_widget(art_paragraph, header[0]);
    f.render_widget(text_paragraph, header[1]);
}

fn render_weather_details(f: &mut Frame, panel: &WeatherPanelData, area: Rect, palette: &Palette) {
    if !panel.has_forecast() {
        return;
    }

    if area.width > 100 {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(45), Constraint::Min(0)])
            .split(area);
        render_forecast_list(f, panel, columns[0], palette);
        render_temperature_chart(f, panel, columns[1], palette);
    } else {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(12), Constraint::Min(0)])
            .split(area);
        render_forecast_list(f, panel, rows[0], palette);
        render_temperature_chart(f, panel, rows[1], palette);
    }
}

fn render_forecast_list(f: &mut Frame, panel: &WeatherPanelData, area: Rect, palette: &Palette) {
    let items = panel
        .forecast_rows
        .iter()
        .map(|row| {
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {} ", row.time_label),
                    Style::default().fg(palette.secondary_accent),
                ),
                Span::raw(" | "),
                Span::styled(row.icon, Style::default().fg(row.icon_color)),
                Span::styled(&row.cloud_label, Style::default().fg(palette.text_muted)),
                Span::raw(" | "),
                Span::styled(
                    &row.temperature_label,
                    Style::default()
                        .fg(palette.accent)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
        })
        .collect::<Vec<_>>();

    let forecast_list = List::new(items).block(
        Block::default()
            .borders(Borders::LEFT)
            .border_style(Style::default().fg(palette.border_inactive))
            .title(Span::styled(
                " 24h Forecast ",
                Style::default()
                    .fg(palette.fg)
                    .add_modifier(Modifier::BOLD),
            )),
    );
    f.render_widget(forecast_list, area);
}

fn render_temperature_chart(
    f: &mut Frame,
    panel: &WeatherPanelData,
    area: Rect,
    palette: &Palette,
) {
    let dataset = Dataset::default()
        .name("Temp")
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(palette.accent))
        .data(&panel.temperature_chart.points);
    let chart = Chart::new(vec![dataset])
        .block(
            Block::default()
                .title(Span::styled(
                    " 24h Temperature Trend ",
                    Style::default()
                        .fg(palette.fg)
                        .add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::LEFT)
                .border_style(Style::default().fg(palette.border_inactive)),
        )
        .x_axis(
            Axis::default()
                .style(Style::default().fg(palette.text_muted))
                .bounds([0.0, 8.0]),
        )
        .y_axis(
            Axis::default()
                .title("°C")
                .style(Style::default().fg(palette.text_muted))
                .bounds([
                    panel.temperature_chart.min_temp,
                    panel.temperature_chart.max_temp,
                ])
                .labels(vec![
                    Span::raw(&panel.temperature_chart.min_label),
                    Span::raw(&panel.temperature_chart.mid_label),
                    Span::raw(&panel.temperature_chart.max_label),
                ]),
        );
    f.render_widget(chart, area);
}
