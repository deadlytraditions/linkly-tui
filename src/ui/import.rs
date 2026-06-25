//! CSV import screen: file browser → preview → running → done (+ QR offer).

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::forms::import::{ImportStage, ImportState, ParsedImport, Progress, Summary, TemplatePicker, SUPPORTED};
use crate::ui::{panel, status_bar, theme, with_status_bar};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let (main, status) = with_status_bar(frame.area());

    let help = match app.import.as_ref().map(|s| &s.stage) {
        Some(ImportStage::Browse) => {
            "↑↓ move · Enter open/select · Backspace up · t template · ? help · Esc cancel"
        }
        Some(ImportStage::TemplateSelect(_)) => "↑↓ move · Space toggle · Enter write · Esc back",
        Some(ImportStage::Preview(_)) => "Enter/y import · Esc back",
        Some(ImportStage::Running(_)) => "importing…",
        Some(ImportStage::Done(_)) => "y QR the new links · Esc/Enter back to list",
        None => "",
    };

    let ws = app.workspace_label();
    if let Some(state) = app.import.as_mut() {
        match &state.stage {
            ImportStage::Browse => render_browse(frame, main, state, &ws),
            ImportStage::TemplateSelect(p) => render_template_select(frame, main, p, &ws),
            ImportStage::Preview(p) => render_preview(frame, main, p, &ws),
            ImportStage::Running(p) => render_running(frame, main, p, &ws),
            ImportStage::Done(sum) => render_done(frame, main, sum, &ws),
        }
    }

    status_bar(frame, status, app, help);
}

fn render_template_select(frame: &mut Frame, area: Rect, picker: &TemplatePicker, ws: &str) {
    use ratatui::widgets::ListState;

    let items: Vec<ListItem> = SUPPORTED
        .iter()
        .zip(&picker.selected)
        .map(|(col, &sel)| {
            let (mark, mark_style) = if sel {
                ("[x] ", Style::default().fg(theme::OK))
            } else {
                ("[ ] ", Style::default().fg(theme::MUTED))
            };
            ListItem::new(Line::from(vec![
                Span::styled(mark, mark_style),
                Span::styled(
                    (*col).to_string(),
                    Style::default().fg(if sel { Color::White } else { theme::MUTED }),
                ),
            ]))
        })
        .collect();

    let chosen = picker.selected.iter().filter(|s| **s).count();
    let list = List::new(items)
        .block(panel(&format!(
            "{ws} · Template columns ({chosen} selected) → linkly-import-template.csv"
        )))
        .highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▍ ");

    let mut state = ListState::default();
    state.select(Some(picker.cursor));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_browse(frame: &mut Frame, area: Rect, state: &mut ImportState, ws: &str) {
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

    let items: Vec<ListItem> = state
        .browser
        .entries
        .iter()
        .map(|e| {
            if e.is_dir {
                ListItem::new(Line::from(Span::styled(
                    format!("{}/", e.name),
                    Style::default().fg(theme::ACCENT),
                )))
            } else {
                ListItem::new(Line::from(Span::styled(
                    e.name.clone(),
                    Style::default().fg(Color::White),
                )))
            }
        })
        .collect();

    let title = format!("{ws} · Import CSV · {}", state.browser.dir.display());
    let list = List::new(items)
        .block(panel(&title))
        .highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▍ ");
    frame.render_stateful_widget(list, rows[0], &mut state.browser.state);

    let msg = state.message.clone().unwrap_or_else(|| {
        "Columns = field names (url required). Press t to drop a template.".to_string()
    });
    frame.render_widget(
        Paragraph::new(Span::styled(msg, Style::default().fg(theme::MUTED))),
        rows[1],
    );
}

fn render_preview(frame: &mut Frame, area: Rect, p: &ParsedImport, ws: &str) {
    let file = p
        .path
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Rows: ", Style::default().fg(theme::MUTED)),
            Span::styled(p.total().to_string(), Style::default().fg(Color::White)),
            Span::styled("   valid: ", Style::default().fg(theme::MUTED)),
            Span::styled(p.valid().to_string(), Style::default().fg(theme::OK)),
            Span::styled("   invalid: ", Style::default().fg(theme::MUTED)),
            Span::styled(
                p.invalid().to_string(),
                Style::default().fg(if p.invalid() > 0 { theme::ERROR } else { theme::MUTED }),
            ),
        ]),
        Line::from(""),
    ];

    for w in &p.warnings {
        lines.push(Line::from(Span::styled(
            format!("⚠ {w}"),
            Style::default().fg(Color::Yellow),
        )));
    }
    if !p.warnings.is_empty() {
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled(
        "Sample:",
        Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
    )));
    for row in p.rows.iter().take(8) {
        let line = match &row.error {
            Some(err) => Line::from(vec![
                Span::styled("  ✗ ", Style::default().fg(theme::ERROR)),
                Span::styled(format!("line {}: ", row.line), Style::default().fg(theme::MUTED)),
                Span::styled(err.clone(), Style::default().fg(theme::ERROR)),
            ]),
            None => {
                let slug = if row.slug_display.is_empty() {
                    String::new()
                } else {
                    format!("  ({})", row.slug_display)
                };
                Line::from(vec![
                    Span::styled("  ✓ ", Style::default().fg(theme::OK)),
                    Span::styled(row.url_display.clone(), Style::default().fg(Color::White)),
                    Span::styled(slug, Style::default().fg(theme::ACCENT)),
                ])
            }
        };
        lines.push(line);
    }
    if p.rows.len() > 8 {
        lines.push(Line::from(Span::styled(
            format!("  … and {} more", p.rows.len() - 8),
            Style::default().fg(theme::MUTED),
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("Press Enter to create {} link(s).", p.valid()),
        Style::default().fg(theme::OK).add_modifier(Modifier::BOLD),
    )));

    frame.render_widget(
        Paragraph::new(lines).block(panel(&format!("{ws} · Preview · {file}"))),
        area,
    );
}

fn render_running(frame: &mut Frame, area: Rect, p: &Progress, ws: &str) {
    let rows = Layout::vertical([Constraint::Length(3), Constraint::Min(0)])
        .horizontal_margin(1)
        .split(area);

    let ratio = if p.total == 0 {
        1.0
    } else {
        p.done as f64 / p.total as f64
    };
    let gauge = Gauge::default()
        .block(panel(&format!("{ws} · Importing")))
        .gauge_style(Style::default().fg(theme::ACCENT))
        .ratio(ratio)
        .label(format!(
            "{}/{}  ·  ok {}  ·  failed {}",
            p.done, p.total, p.ok, p.failed
        ));
    frame.render_widget(gauge, rows[0]);
}

fn render_done(frame: &mut Frame, area: Rect, sum: &Summary, ws: &str) {
    let mut lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("Created: ", Style::default().fg(theme::MUTED)),
            Span::styled(sum.ok.to_string(), Style::default().fg(theme::OK)),
            Span::styled("    Failed: ", Style::default().fg(theme::MUTED)),
            Span::styled(
                sum.failed.to_string(),
                Style::default().fg(if sum.failed > 0 { theme::ERROR } else { theme::MUTED }),
            ),
        ]),
        Line::from(""),
    ];

    if let Some(p) = &sum.success_path {
        lines.push(Line::from(Span::styled(
            format!("✓ wrote {}", p.display()),
            Style::default().fg(theme::MUTED),
        )));
    }
    if let Some(p) = &sum.failure_path {
        lines.push(Line::from(Span::styled(
            format!("✗ wrote {}", p.display()),
            Style::default().fg(theme::MUTED),
        )));
    }
    lines.push(Line::from(""));

    match sum.qr_done {
        Some(n) => {
            let where_ = sum.qr_dir.clone().unwrap_or_default();
            lines.push(Line::from(Span::styled(
                format!("Generated {n} QR code(s) in {where_}/"),
                Style::default().fg(theme::OK),
            )));
            lines.push(Line::from(Span::styled(
                "Press Esc to return to the list.",
                Style::default().fg(theme::MUTED),
            )));
        }
        None if !sum.new_links.is_empty() => {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("Generate QR codes for the {} new link(s)? ", sum.new_links.len()),
                    Style::default().fg(Color::White),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("[y]", Style::default().fg(theme::OK)),
                Span::styled(" yes    ", Style::default().fg(theme::MUTED)),
                Span::styled("[n]/Esc", Style::default().fg(theme::ACCENT)),
                Span::styled(" no", Style::default().fg(theme::MUTED)),
            ]));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "Press Esc to return to the list.",
                Style::default().fg(theme::MUTED),
            )));
        }
    }

    frame.render_widget(
        Paragraph::new(lines).block(panel(&format!("{ws} · Import complete"))),
        area,
    );
}
