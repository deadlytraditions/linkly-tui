//! Link list screen: a table of links with click stats.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{panel, status_bar, theme, with_status_bar};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let (main, status) = with_status_bar(frame.area());

    // When searching, reserve a line above the status bar for the query input.
    let (table_area, search_area) = if app.searching {
        let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(main);
        (rows[0], Some(rows[1]))
    } else {
        (main, None)
    };

    let header = Row::new(vec![
        Cell::from(" Name"),
        Cell::from("Slug"),
        Cell::from("Domain"),
        Cell::from("Total"),
        Cell::from("30d"),
        Cell::from("Today"),
        Cell::from("On"),
    ])
    .style(
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )
    .bottom_margin(1);

    let rows = app.links.iter().map(|l| {
        let enabled = match l.enabled {
            Some(true) => Span::styled("●", Style::default().fg(theme::OK)),
            Some(false) => Span::styled("○", Style::default().fg(theme::ERROR)),
            None => Span::styled("·", Style::default().fg(theme::MUTED)),
        };
        Row::new(vec![
            Cell::from(Span::styled(
                format!(" {}", l.name.clone().unwrap_or_default()),
                Style::default().fg(Color::White),
            )),
            Cell::from(Span::styled(
                l.slug.clone().unwrap_or_default(),
                Style::default().fg(theme::ACCENT),
            )),
            Cell::from(Span::styled(
                l.domain.clone().unwrap_or_default(),
                Style::default().fg(Color::Gray),
            )),
            Cell::from(l.clicks_total.to_string()),
            Cell::from(l.clicks_thirty_days.to_string()),
            Cell::from(l.clicks_today.to_string()),
            Cell::from(enabled),
        ])
    });

    let widths = [
        Constraint::Percentage(26),
        Constraint::Percentage(20),
        Constraint::Percentage(24),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(7),
        Constraint::Length(3),
    ];

    let title = if app.search.is_empty() {
        format!("Links · workspace {}", app.workspace_id)
    } else {
        format!("Links · workspace {} · search “{}”", app.workspace_id, app.search)
    };

    let table = Table::new(rows, widths)
        .header(header)
        .block(panel(&title))
        .row_highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(Span::styled("▍", Style::default().fg(theme::ACCENT)));

    frame.render_stateful_widget(table, table_area, &mut app.list_state);

    if let Some(area) = search_area {
        render_search(frame, area, app);
    }

    status_bar(
        frame,
        status,
        app,
        "↑↓ move · Enter details · c create · / search · n/p page · r refresh · q quit",
    );
}

fn render_search(frame: &mut Frame, area: Rect, app: &App) {
    let line = Paragraph::new(Span::styled(
        format!(" search ▍ {}▏", app.search_input.value()),
        Style::default().fg(Color::Yellow),
    ));
    frame.render_widget(line, area);
}
