//! Link list screen: a table of links with click stats.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table};
use ratatui::Frame;

use crate::app::{App, SortField};
use crate::ui::{centered_rect, input_spans, panel, status_bar, theme, with_status_bar};

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

    let arrow = if app.sort_desc { "↓" } else { "↑" };
    let search_note = if app.search.is_empty() {
        String::new()
    } else {
        format!(" · search “{}”", app.search)
    };
    let title = format!(
        "Links · ws {} · sort {} {} · page {}/{} · {} total{}",
        app.workspace_id,
        app.sort_field.label(),
        arrow,
        app.page,
        app.total_pages,
        app.total_entries,
        search_note,
    );

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
        "↑↓ move · Enter details · c create · / search · s sort · n/p page · r refresh · q quit",
    );

    if app.sort_open {
        render_sort_popup(frame, app);
    }
}

fn render_sort_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 55, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = SortField::ALL
        .iter()
        .map(|f| {
            let current = *f == app.sort_field;
            let marker = if current { "● " } else { "  " };
            ListItem::new(Line::from(vec![
                Span::styled(marker, Style::default().fg(theme::ACCENT)),
                Span::styled(f.label(), Style::default().fg(Color::White)),
            ]))
        })
        .collect();

    let dir = if app.sort_cursor_desc {
        "descending ↓"
    } else {
        "ascending ↑"
    };
    let title = format!("Sort by · {dir}");

    let list = List::new(items)
        .block(panel(&title))
        .highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▍ ");

    let mut state = ListState::default();
    state.select(Some(app.sort_cursor));
    frame.render_stateful_widget(list, area, &mut state);

    // Footer hint inside the popup's bottom border row.
    let hint_area = Rect {
        x: area.x + 2,
        y: area.y + area.height.saturating_sub(1),
        width: area.width.saturating_sub(4),
        height: 1,
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            " ↑↓ field · d/←→ direction · Enter apply · Esc cancel ",
            Style::default().fg(theme::MUTED),
        )),
        hint_area,
    );
}

fn render_search(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![Span::styled(
        " search ▍ ",
        Style::default().fg(theme::MUTED),
    )];
    spans.extend(input_spans(
        &app.search_input,
        false,
        Style::default().fg(Color::Yellow),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}
