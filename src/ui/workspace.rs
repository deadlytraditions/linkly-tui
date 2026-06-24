//! Startup workspace picker: choose a remembered workspace or add a new one.

use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_horizontal, panel, render_banner, theme, BANNER};

pub fn draw(frame: &mut Frame, app: &App) {
    let full = frame.area();
    let banner_h = BANNER.len() as u16;

    let rows = Layout::vertical([
        Constraint::Length(1),        // top padding
        Constraint::Length(banner_h), // banner (full width)
        Constraint::Length(1),        // subtitle
        Constraint::Length(1),        // spacer
        Constraint::Min(3),           // table
        Constraint::Length(1),        // footer
    ])
    .split(full);

    render_banner(frame, rows[1]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "choose a workspace",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center),
        rows[2],
    );

    let table_area = centered_horizontal(rows[4], 72);

    let header = Row::new(vec![
        Cell::from("Workspace"),
        Cell::from("ID"),
        Cell::from("Key"),
    ])
    .style(
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let mut table_rows: Vec<Row> = app
        .cached_workspaces
        .iter()
        .map(|w| {
            let key = if w.api_key.is_some() {
                Span::styled("🔑 saved", Style::default().fg(theme::OK))
            } else {
                Span::styled("—", Style::default().fg(theme::MUTED))
            };
            Row::new(vec![
                Cell::from(Span::styled(w.name.clone(), Style::default().fg(Color::White))),
                Cell::from(Span::styled(
                    w.id.to_string(),
                    Style::default().fg(theme::MUTED),
                )),
                Cell::from(key),
            ])
        })
        .collect();

    // Trailing "add new" row.
    table_rows.push(Row::new(vec![
        Cell::from(Span::styled(
            "+ Add new workspace",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )),
        Cell::from(""),
        Cell::from(""),
    ]));

    let widths = [
        Constraint::Min(20),
        Constraint::Length(10),
        Constraint::Length(10),
    ];

    let table = Table::new(table_rows, widths)
        .header(header)
        .block(panel("Workspaces"))
        .row_highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(Span::styled("▍ ", Style::default().fg(theme::ACCENT)));

    let mut state = TableState::default();
    state.select(Some(app.picker_cursor));
    frame.render_stateful_widget(table, table_area, &mut state);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "↑↓ select · Enter continue · d forget (+ stored key) · Esc/q quit",
            Style::default().fg(theme::MUTED),
        ))
        .alignment(Alignment::Center),
        rows[5],
    );
}
