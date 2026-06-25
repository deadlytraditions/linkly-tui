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

    let widths = [
        Constraint::Percentage(36), // Name (wider)
        Constraint::Percentage(10), // Slug (halved)
        Constraint::Percentage(24), // Domain
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
        "{} · Links · Sorted by: {} {} · page {}/{} · {} total{}",
        app.workspace_label(),
        app.sort_field.label(),
        arrow,
        app.page,
        app.total_pages,
        app.total_entries,
        search_note,
    );

    let block = panel(&title);

    // Work out the actual width of the Name column using the same layout the
    // table uses, so we can truncate over-long names with an ellipsis. The table
    // reserves one column on the left for the selection symbol "▍".
    let inner = block.inner(table_area);
    let columns_area = Rect {
        width: inner.width.saturating_sub(1),
        ..inner
    };
    let col_widths = Layout::horizontal(widths).spacing(1).split(columns_area);
    // Names get a leading space, so the usable text width is one less.
    let name_avail = (col_widths[0].width as usize).saturating_sub(1);

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
        let name = truncate_ellipsis(l.name.as_deref().unwrap_or_default(), name_avail);
        Row::new(vec![
            Cell::from(Span::styled(
                format!(" {name}"),
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

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
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
        "/ search · ? help · q quit",
    );

    if app.sort_open {
        render_sort_popup(frame, app);
    }
    if app.store_prompt {
        render_store_prompt(frame, app);
    }
}

fn render_store_prompt(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 42, frame.area());
    frame.render_widget(Clear, area);

    let block = panel("Store API key?");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let name = app
        .cached_workspaces
        .iter()
        .find(|w| w.id == app.workspace_id)
        .map(|w| w.name.clone())
        .unwrap_or_else(|| format!("workspace {}", app.workspace_id));

    let rows = Layout::vertical([
        Constraint::Length(2), // question
        Constraint::Length(2), // warning
        Constraint::Length(1), // path
        Constraint::Length(1), // spacer
        Constraint::Length(1), // actions
    ])
    .horizontal_margin(2)
    .split(inner);

    frame.render_widget(
        Paragraph::new(format!(
            "Save this API key for “{name}” so you won't re-enter it?"
        ))
        .style(Style::default().fg(Color::White)),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new("⚠ The key is stored in PLAINTEXT — anyone who can read")
            .style(Style::default().fg(theme::ERROR)),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new("  the cache file can use your Linkly account.")
            .style(Style::default().fg(theme::ERROR)),
        rows[2],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[s]", Style::default().fg(theme::OK)),
            Span::styled(" store   ", Style::default().fg(theme::MUTED)),
            Span::styled("[n]", Style::default().fg(theme::ACCENT)),
            Span::styled(" not now / Esc", Style::default().fg(theme::MUTED)),
        ])),
        rows[4],
    );
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

/// Truncate `s` to at most `max` display columns, appending `…` when cut so it's
/// clear the value is incomplete.
fn truncate_ellipsis(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    match max {
        0 => String::new(),
        1 => "…".to_string(),
        _ => {
            let kept: String = s.chars().take(max - 1).collect();
            format!("{kept}…")
        }
    }
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

#[cfg(test)]
mod tests {
    use super::truncate_ellipsis;

    #[test]
    fn fits_unchanged() {
        assert_eq!(truncate_ellipsis("promo", 10), "promo");
        assert_eq!(truncate_ellipsis("promo", 5), "promo");
    }

    #[test]
    fn cut_adds_ellipsis_and_respects_width() {
        let out = truncate_ellipsis("summer-promo-2026", 8);
        assert_eq!(out, "summer-…");
        assert_eq!(out.chars().count(), 8);
    }

    #[test]
    fn tiny_widths() {
        assert_eq!(truncate_ellipsis("abc", 1), "…");
        assert_eq!(truncate_ellipsis("abc", 0), "");
    }
}
