//! Link detail screen: every populated field, with values aligned in a column.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use serde_json::Value;

use crate::app::App;
use crate::ui::{panel, status_bar, theme, with_status_bar};

/// Upper bound on the key column so one very long key can't push every value
/// far to the right.
const MAX_KEY_WIDTH: usize = 24;

pub fn draw(frame: &mut Frame, app: &App) {
    let (main, status) = with_status_bar(frame.area());

    let title = match app.selected_link().and_then(|l| l.id) {
        Some(id) => format!("Link #{id}"),
        None => "Link details".to_string(),
    };

    let lines = match &app.detail {
        Some(Value::Object(map)) => render_fields(map),
        Some(_) => vec![Line::from("Unexpected response shape.")],
        None => vec![Line::from(Span::styled(
            "Loading…",
            Style::default().fg(theme::MUTED),
        ))],
    };

    let paragraph = Paragraph::new(lines)
        .block(panel(&title))
        .scroll((app.detail_scroll, 0));
    frame.render_widget(paragraph, main);

    status_bar(frame, status, app, "↑↓ scroll · Esc back");
}

fn render_fields(map: &serde_json::Map<String, Value>) -> Vec<Line<'static>> {
    // Collect the non-empty fields first so the column width is based only on
    // what we actually show.
    let mut entries: Vec<(&String, &Value)> = map
        .iter()
        .filter(|(_, v)| !is_empty(v))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let key_width = entries
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0)
        .min(MAX_KEY_WIDTH);

    entries
        .into_iter()
        .map(|(k, v)| {
            let (value, style) = render_value(v);
            Line::from(vec![
                Span::styled(
                    format!("{k:<key_width$} ", key_width = key_width),
                    Style::default().fg(theme::ACCENT),
                ),
                Span::styled("│ ", Style::default().fg(theme::BORDER)),
                Span::styled(value, style),
            ])
        })
        .collect()
}

fn is_empty(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// Render a JSON value to a single-line string plus a style hint.
fn render_value(v: &Value) -> (String, Style) {
    match v {
        Value::String(s) => (s.clone(), Style::default().fg(Color::White)),
        Value::Bool(true) => ("true".into(), Style::default().fg(theme::OK)),
        Value::Bool(false) => ("false".into(), Style::default().fg(theme::MUTED)),
        Value::Number(n) => (
            n.to_string(),
            Style::default()
                .fg(theme::ACCENT_DIM)
                .add_modifier(Modifier::BOLD),
        ),
        other => (other.to_string(), Style::default().fg(Color::Gray)),
    }
}
