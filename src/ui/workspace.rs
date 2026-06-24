//! Startup workspace picker: choose a remembered workspace or add a new one.

use ratatui::layout::{Alignment, Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_rect, panel, render_banner, theme, BANNER};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = centered_rect(64, 88, frame.area());

    let rows = Layout::vertical([
        Constraint::Length(BANNER.len() as u16), // banner
        Constraint::Length(2),                    // subtitle
        Constraint::Min(3),                       // list
        Constraint::Length(1),                    // footer
    ])
    .split(area);

    render_banner(frame, rows[0]);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "choose a workspace",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );

    let mut items: Vec<ListItem> = app
        .cached_workspaces
        .iter()
        .map(|w| {
            ListItem::new(Line::from(vec![
                Span::styled(w.name.clone(), Style::default().fg(Color::White)),
                Span::styled(
                    format!("  ·  id {}", w.id),
                    Style::default().fg(theme::MUTED),
                ),
            ]))
        })
        .collect();
    items.push(ListItem::new(Span::styled(
        "+ Add new workspace",
        Style::default()
            .fg(theme::ACCENT)
            .add_modifier(Modifier::BOLD),
    )));

    let list = List::new(items)
        .block(panel("Workspaces"))
        .highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▍ ");

    let mut state = ListState::default();
    state.select(Some(app.picker_cursor));
    frame.render_stateful_widget(list, rows[2], &mut state);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "↑↓ select · Enter continue · d forget · Esc/q quit",
            Style::default().fg(theme::MUTED),
        ))
        .alignment(Alignment::Center),
        rows[3],
    );
}
