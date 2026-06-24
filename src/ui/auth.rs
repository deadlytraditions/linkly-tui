//! Startup credential prompt. The API key is rendered masked. When a workspace
//! was picked from the cache, only the API key is requested.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use tui_input::Input;

use crate::app::App;
use crate::ui::{centered_horizontal, input_spans, render_banner, theme, BANNER};

pub fn draw(frame: &mut Frame, app: &App) {
    let full = frame.area();
    let locked = app.auth.ws_locked;

    // The form box is shorter when only the API key is requested.
    let form_height = if locked { 6 } else { 8 };

    let rows = Layout::vertical([
        Constraint::Length(1),                    // top padding
        Constraint::Length(BANNER.len() as u16),  // banner (full width)
        Constraint::Length(2),                    // subtitle
        Constraint::Length(1),                    // spacer
        Constraint::Length(form_height),          // form box
        Constraint::Length(1),                    // spacer
        Constraint::Length(1),                    // footer
        Constraint::Min(0),                       // filler
    ])
    .split(full);

    render_banner(frame, rows[1]);

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
        rows[2],
    );

    // Centre the credentials box horizontally.
    let form_box = centered_horizontal(rows[4], 48);
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " credentials ",
            Style::default().fg(theme::ACCENT),
        ));
    let inner = block.inner(form_box);
    frame.render_widget(block, form_box);

    let mut constraints = vec![Constraint::Length(2)]; // api key
    if !locked {
        constraints.push(Constraint::Length(2)); // workspace id
    }
    constraints.push(Constraint::Length(1)); // error
    let form = Layout::vertical(constraints)
        .horizontal_margin(2)
        .split(inner);

    field(
        frame,
        form[0],
        "API key",
        &app.auth.api_key,
        true,
        app.auth.focus == 0,
    );

    let error_row = if locked {
        form[1]
    } else {
        field(
            frame,
            form[1],
            "Workspace ID",
            &app.auth.workspace_id,
            false,
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
        rows[6],
    );
}

fn field(frame: &mut Frame, area: Rect, label: &str, input: &Input, masked: bool, focused: bool) {
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

    let mut value_spans = vec![Span::raw("  ")];
    if focused {
        value_spans.extend(input_spans(input, masked, value_style));
    } else {
        let v = if masked {
            "•".repeat(input.value().chars().count())
        } else {
            input.value().to_string()
        };
        value_spans.push(Span::styled(v, value_style));
    }

    let lines = vec![
        Line::from(vec![
            Span::styled(marker, Style::default().fg(theme::ACCENT)),
            Span::styled(label.to_string(), label_style),
        ]),
        Line::from(value_spans),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}
