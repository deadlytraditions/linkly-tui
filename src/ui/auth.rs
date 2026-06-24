//! Startup credential prompt. The API key is rendered masked. When a workspace
//! was picked from the cache, only the API key is requested.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_rect, render_banner, theme, BANNER};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = centered_rect(64, 90, frame.area());
    let locked = app.auth.ws_locked;

    // The form box is shorter when only the API key is requested.
    let form_height = if locked { 6 } else { 8 };

    let rows = Layout::vertical([
        Constraint::Length(BANNER.len() as u16), // banner
        Constraint::Length(2),                   // subtitle
        Constraint::Length(form_height),         // form box
        Constraint::Length(1),                   // footer
    ])
    .flex(ratatui::layout::Flex::Center)
    .split(area);

    render_banner(frame, rows[0]);

    let subtitle = if locked {
        format!("workspace · {} (id {})", app.auth.ws_name, app.workspace_id)
    } else {
        "new workspace · enter API key and workspace ID".to_string()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            subtitle,
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );

    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " credentials ",
            Style::default().fg(theme::ACCENT),
        ));
    let inner = block.inner(rows[2]);
    frame.render_widget(block, rows[2]);

    let mut constraints = vec![Constraint::Length(2)]; // api key
    if !locked {
        constraints.push(Constraint::Length(2)); // workspace id
    }
    constraints.push(Constraint::Length(1)); // error
    let form = Layout::vertical(constraints)
        .horizontal_margin(2)
        .split(inner);

    let key_len = app.auth.api_key.value().chars().count();
    field(
        frame,
        form[0],
        "API key",
        &"•".repeat(key_len),
        app.auth.focus == 0,
    );

    let error_row = if locked {
        form[1]
    } else {
        field(
            frame,
            form[1],
            "Workspace ID",
            app.auth.workspace_id.value(),
            app.auth.focus == 1,
        );
        form[2]
    };

    if let Some(e) = &app.auth.error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("⚠ {e}"),
                Style::default().fg(theme::ERROR),
            )),
            error_row,
        );
    }

    let footer = if app.cached_workspaces.is_empty() {
        "Tab switch field · Enter continue · Esc quit"
    } else {
        "Tab switch field · Enter continue · Esc back"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(footer, Style::default().fg(theme::MUTED)))
            .alignment(Alignment::Center),
        rows[3],
    );
}

fn field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
    let (marker, label_style, value_style) = if focused {
        (
            "▍ ",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(Color::White),
        )
    } else {
        (
            "  ",
            Style::default().fg(theme::MUTED),
            Style::default().fg(Color::Gray),
        )
    };
    let cursor = if focused { "▏" } else { "" };
    let lines = vec![
        Line::from(vec![
            Span::styled(marker, Style::default().fg(theme::ACCENT)),
            Span::styled(label.to_string(), label_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(value.to_string(), value_style),
            Span::styled(cursor, Style::default().fg(theme::ACCENT)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}
