//! Link detail / edit screen.
//!
//! Fields are shown in a navigable list (the selected line is highlighted and
//! the list only scrolls to keep it visible). Enter edits the current field;
//! changed fields are marked and the title shows unsaved state.

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState, Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::App;
use crate::forms::edit_form::{DetailMode, EditField, EditKind, LinkEditor};
use crate::ui::{centered_rect, input_spans, panel, status_bar, theme, with_status_bar};

pub fn draw(frame: &mut Frame, app: &App) {
    let (main, status) = with_status_bar(frame.area());

    // The link fields on top, a clicks graph pinned to the bottom.
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(9)]).split(main);
    render_clicks_chart(frame, rows[1], app);
    let main = rows[0];

    let Some(editor) = &app.editor else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Loading…",
                Style::default().fg(theme::MUTED),
            ))
            .block(panel(&format!("{} · Link details", app.workspace_label()))),
            main,
        );
        status_bar(frame, status, app, "Esc back");
        return;
    };

    let editing = editor.mode == DetailMode::Edit;
    let label_width = editor
        .fields
        .iter()
        .map(|f| f.label.len())
        .max()
        .unwrap_or(0);

    let items: Vec<ListItem> = editor
        .fields
        .iter()
        .enumerate()
        .map(|(i, f)| field_item(f, label_width, i == editor.cursor && editing))
        .collect();

    let dirty = if editor.dirty() { " · unsaved ●" } else { "" };
    let title = format!(
        "{} · Link #{} · {}{}",
        app.workspace_label(),
        editor.id,
        editor.full_url,
        dirty
    );

    // Highlight is yellow while editing the current line, blue while navigating.
    let highlight = if editing {
        Style::default()
            .bg(Color::Rgb(90, 75, 30))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(theme::SELECT_BG)
            .add_modifier(Modifier::BOLD)
    };

    let list = List::new(items)
        .block(panel(&title))
        .highlight_style(highlight)
        .highlight_symbol("▍ ");

    let mut state = ListState::default();
    state.select(Some(editor.cursor));
    frame.render_stateful_widget(list, main, &mut state);

    let help = match editor.mode {
        DetailMode::Nav => "Enter edit · ? help · Esc back",
        DetailMode::Edit => "type to edit · Enter/Esc done",
        DetailMode::ConfirmSave => "s save · d discard · Esc cancel",
    };
    status_bar(frame, status, app, help);

    if editor.mode == DetailMode::ConfirmSave {
        render_confirm_popup(frame, editor);
    }
}

/// Render the link's daily clicks (last 30 days) as a graph with a y-axis scale.
fn render_clicks_chart(frame: &mut Frame, area: Rect, app: &App) {
    let all_time = app.selected_link().map(|l| l.clicks_total).unwrap_or(0);

    let Some(series) = &app.detail_clicks else {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "Loading clicks…",
                Style::default().fg(theme::MUTED),
            ))
            .block(panel("Clicks (last 30 days)")),
            area,
        );
        return;
    };

    let data: Vec<u64> = series.iter().map(|(_, y)| (*y).max(0) as u64).collect();
    let sum: i64 = series.iter().map(|(_, y)| *y).sum();
    let max = data.iter().copied().max().unwrap_or(0);

    if max == 0 {
        frame.render_widget(
            Paragraph::new(Span::styled(
                "No clicks recorded in this period.",
                Style::default().fg(theme::MUTED),
            ))
            .block(panel(&format!(
                "Clicks (last 30 days): 0 · all-time: {all_time}"
            ))),
            area,
        );
        return;
    }

    let title =
        format!("Clicks (last 30 days): {sum} · all-time: {all_time} · peak {max}/day");
    let block = panel(&title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Left gutter holds the y-axis scale labels; the rest holds the bars.
    let cols = Layout::horizontal([Constraint::Length(7), Constraint::Min(1)]).split(inner);
    let gutter = cols[0];
    let h = gutter.height;

    let mut tick = |y: u16, value: u64| {
        let rect = Rect {
            x: gutter.x,
            y,
            width: gutter.width,
            height: 1,
        };
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("{value:>4} ┤"),
                Style::default().fg(theme::MUTED),
            )),
            rect,
        );
    };
    tick(gutter.y, max);
    if h >= 3 {
        tick(gutter.y + h / 2, max / 2);
    }
    tick(gutter.y + h.saturating_sub(1), 0);

    let sparkline = Sparkline::default()
        .data(&data)
        .max(max)
        .style(Style::default().fg(theme::ACCENT));
    frame.render_widget(sparkline, cols[1]);
}

fn field_item<'a>(f: &EditField, label_width: usize, editing: bool) -> ListItem<'a> {
    let changed = f.changed();
    let marker = if changed { "*" } else { " " };

    let base = if changed {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::White)
    };

    let value_spans: Vec<Span> = match f.kind {
        EditKind::Bool => {
            if f.bool_val {
                vec![Span::styled("◉ on", Style::default().fg(theme::OK))]
            } else {
                vec![Span::styled("◯ off", Style::default().fg(theme::MUTED))]
            }
        }
        // Field being edited: draw a real caret at the cursor position.
        EditKind::Text if editing => input_spans(&f.input, false, base),
        EditKind::Text if f.input.value().is_empty() => {
            vec![Span::styled("—", Style::default().fg(theme::MUTED))]
        }
        EditKind::Text => vec![Span::styled(f.input.value().to_string(), base)],
    };

    let mut spans = vec![
        Span::styled(format!("{marker} "), Style::default().fg(Color::Yellow)),
        Span::styled(
            format!("{:<label_width$}  ", f.label, label_width = label_width),
            Style::default().fg(theme::ACCENT_DIM),
        ),
    ];
    spans.extend(value_spans);
    ListItem::new(Line::from(spans))
}

fn render_confirm_popup(frame: &mut Frame, _editor: &LinkEditor) {
    let area = centered_rect(46, 28, frame.area());
    frame.render_widget(Clear, area);

    let block = panel("Unsaved changes");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .horizontal_margin(2)
    .split(inner);

    frame.render_widget(
        Paragraph::new(Span::styled(
            "Save your changes before leaving?",
            Style::default().fg(Color::White),
        )),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("[s]", Style::default().fg(theme::OK)),
            Span::styled(" save   ", Style::default().fg(theme::MUTED)),
            Span::styled("[d]", Style::default().fg(theme::ERROR)),
            Span::styled(" discard   ", Style::default().fg(theme::MUTED)),
            Span::styled("[Esc]", Style::default().fg(theme::ACCENT)),
            Span::styled(" cancel", Style::default().fg(theme::MUTED)),
        ])),
        rows[2],
    );
}
