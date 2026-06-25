//! Rendering. `draw` routes to the active screen's renderer.

mod auth;
mod create;
mod detail;
mod import;
mod list;
mod workspace;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use tui_input::Input;

use crate::app::{App, Screen};

/// Shared brand banner, rendered on the startup screens.
pub const BANNER: &[&str] = &[
    r"  ██╗     ██╗███╗   ██╗██╗  ██╗██╗  ██╗   ██╗████████╗██╗   ██╗██╗",
    r"  ██║     ██║████╗  ██║██║ ██╔╝██║  ╚██╗ ██╔╝╚══██╔══╝██║   ██║██║",
    r"  ██║     ██║██╔██╗ ██║█████╔╝ ██║   ╚████╔╝    ██║   ██║   ██║██║",
    r"  ██║     ██║██║╚██╗██║██╔═██╗ ██║    ╚██╔╝     ██║   ██║   ██║██║",
    r"  ███████╗██║██║ ╚████║██║  ██╗███████╗██║      ██║   ╚██████╔╝██║",
    r"  ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝      ╚═╝    ╚═════╝ ╚═╝",
];

/// Render the brand banner centred in `area`.
pub fn render_banner(frame: &mut Frame, area: Rect) {
    let lines: Vec<Line> = BANNER
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(theme::ACCENT))))
        .collect();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

/// Shared colour palette so every screen feels like one product.
pub mod theme {
    use ratatui::style::Color;

    /// Brand accent (periwinkle blue).
    pub const ACCENT: Color = Color::Rgb(120, 150, 255);
    /// Dimmed accent for secondary emphasis.
    pub const ACCENT_DIM: Color = Color::Rgb(96, 120, 210);
    /// Selection background.
    pub const SELECT_BG: Color = Color::Rgb(44, 54, 92);
    /// Subtle border colour.
    pub const BORDER: Color = Color::Rgb(74, 82, 112);
    /// Muted text (help, captions).
    pub const MUTED: Color = Color::Rgb(128, 136, 158);
    pub const ERROR: Color = Color::Rgb(240, 104, 104);
    pub const OK: Color = Color::Rgb(120, 200, 140);
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::WorkspacePicker => workspace::draw(frame, app),
        Screen::Auth => auth::draw(frame, app),
        Screen::LinkList => list::draw(frame, app),
        Screen::LinkDetail => detail::draw(frame, app),
        Screen::CreateLink => create::draw(frame, app),
        Screen::Import => import::draw(frame, app),
    }

    // The QR dialog overlays whatever screen is active.
    if app.qr_settings_open {
        render_qr_dialog(frame, app);
    }
    if app.help_open {
        render_help(frame, app);
    }
}

/// Keybindings shown in the help overlay, per screen.
fn screen_keybinds(screen: Screen) -> &'static [(&'static str, &'static str)] {
    match screen {
        Screen::LinkList => &[
            ("↑/↓  j/k", "move selection"),
            ("Enter", "open link details"),
            ("c", "create a link"),
            ("i", "import links from CSV"),
            ("Q", "export QR codes (whole workspace)"),
            ("o", "edit QR defaults"),
            ("/", "search"),
            ("s", "sort"),
            ("n / p", "next / previous page"),
            ("r", "refresh"),
            ("Esc", "back to workspaces"),
            ("q", "quit"),
        ],
        Screen::LinkDetail => &[
            ("↑/↓", "move between fields"),
            ("Enter", "edit field / toggle"),
            ("s", "save changes"),
            ("Q", "export QR for this link"),
            ("Esc", "back (prompts if unsaved)"),
        ],
        Screen::WorkspacePicker => &[
            ("↑/↓", "select workspace"),
            ("Enter", "continue"),
            ("d", "forget workspace (+ stored key)"),
            ("Esc / q", "quit"),
        ],
        _ => &[],
    }
}

fn render_help(frame: &mut Frame, app: &App) {
    let binds = screen_keybinds(app.screen);
    if binds.is_empty() {
        return;
    }
    let key_w = binds.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    let full = frame.area();
    let width = 52u16.min(full.width.saturating_sub(2));
    let height = (binds.len() as u16 + 3).min(full.height.saturating_sub(2));
    let area = Rect {
        x: full.x + (full.width.saturating_sub(width)) / 2,
        y: full.y + (full.height.saturating_sub(height)) / 2,
        width,
        height,
    };
    frame.render_widget(Clear, area);

    let mut lines: Vec<Line> = binds
        .iter()
        .map(|(k, desc)| {
            Line::from(vec![
                Span::styled(
                    format!("  {k:<key_w$}   "),
                    Style::default()
                        .fg(theme::ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled((*desc).to_string(), Style::default().fg(Color::White)),
            ])
        })
        .collect();
    lines.push(Line::from(Span::styled(
        "  press ? again or Esc to close",
        Style::default().fg(theme::MUTED),
    )));

    frame.render_widget(Paragraph::new(lines).block(panel("Keys")), area);
}

/// The QR export/settings dialog (a global overlay).
fn render_qr_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 45, frame.area());
    frame.render_widget(Clear, area);

    let title = match app.qr_export_label() {
        Some(what) => format!("Export QR · {what}"),
        None => "QR settings".to_string(),
    };
    let block = panel(&title);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::vertical([
        Constraint::Length(1), // format
        Constraint::Length(1), // size
        Constraint::Length(1), // fg
        Constraint::Length(1), // bg
        Constraint::Min(0),    // spacer
        Constraint::Length(1), // hint
    ])
    .horizontal_margin(2)
    .split(inner);

    let label = |text: &str, focused: bool| {
        let style = if focused {
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::ACCENT_DIM)
        };
        Span::styled(format!("{:<8}", format!("{text}:")), style)
    };
    let pointer = |focused: bool| {
        if focused {
            Span::styled("▍ ", Style::default().fg(theme::ACCENT))
        } else {
            Span::raw("  ")
        }
    };

    // Format (cycled, not typed).
    let f0 = app.qr_form_focus == 0;
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            pointer(f0),
            label("Format", f0),
            Span::styled(
                format!("‹ {} ›", app.qr_settings.format.label()),
                Style::default().fg(Color::White),
            ),
        ])),
        rows[0],
    );

    for (i, (text, input)) in [
        ("Size", &app.qr_size_input),
        ("Fg", &app.qr_fg_input),
        ("Bg", &app.qr_bg_input),
    ]
    .into_iter()
    .enumerate()
    {
        let focus = app.qr_form_focus == i + 1;
        let mut spans = vec![pointer(focus), label(text, focus)];
        if focus {
            spans.extend(input_spans(input, false, Style::default().fg(Color::White)));
        } else {
            spans.push(Span::styled(
                input.value().to_string(),
                Style::default().fg(Color::Gray),
            ));
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), rows[i + 1]);
    }

    let confirm = if app.qr_export_label().is_some() {
        "Enter export"
    } else {
        "Enter save"
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!("↑↓ field · ←→ format · type to edit · {confirm} · Esc cancel"),
            Style::default().fg(theme::MUTED),
        )),
        rows[5],
    );
}

/// Split an area into a main region and a one-line status bar at the bottom.
pub fn with_status_bar(area: Rect) -> (Rect, Rect) {
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    (chunks[0], chunks[1])
}

/// Render the shared status/help line: status on the left (state-coloured),
/// key hints on the right.
pub fn status_bar(frame: &mut Frame, area: Rect, app: &App, help: &str) {
    let cols = Layout::horizontal([Constraint::Min(10), Constraint::Length(help.len() as u16 + 1)])
        .split(area);

    let (glyph, style) = if app.loading {
        ("◐ ", Style::default().fg(theme::ACCENT))
    } else if app.status.starts_with("Error") {
        ("✗ ", Style::default().fg(theme::ERROR))
    } else if app.status.is_empty() {
        ("", Style::default())
    } else {
        ("› ", Style::default().fg(theme::OK))
    };
    let status = Line::from(vec![
        Span::styled(glyph, style),
        Span::styled(app.status.clone(), style),
    ]);
    frame.render_widget(Paragraph::new(status), cols[0]);

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            help,
            Style::default().fg(theme::MUTED),
        )))
        .right_aligned(),
        cols[1],
    );
}

/// A bordered block in the house style, with an accented title.
pub fn panel(title: &str) -> ratatui::widgets::Block<'static> {
    use ratatui::widgets::{Block, BorderType};
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            format!(" {title} "),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ))
}

/// Render an editable text input as spans, drawing a block cursor at the real
/// caret position so it's always clear where typing lands — even when the caret
/// is in the middle of the text. Works inside scrolling lists (no screen-coord
/// maths needed). When `masked`, characters are shown as bullets.
pub fn input_spans(input: &Input, masked: bool, base: Style) -> Vec<Span<'static>> {
    let value: String = if masked {
        "•".repeat(input.value().chars().count())
    } else {
        input.value().to_string()
    };
    let chars: Vec<char> = value.chars().collect();
    let cursor = input.visual_cursor().min(chars.len());
    let cursor_style = Style::default()
        .fg(Color::Black)
        .bg(Color::White)
        .add_modifier(Modifier::BOLD);

    let mut spans = Vec::new();
    let before: String = chars[..cursor].iter().collect();
    if !before.is_empty() {
        spans.push(Span::styled(before, base));
    }
    if cursor < chars.len() {
        let at: String = chars[cursor..=cursor].iter().collect();
        spans.push(Span::styled(at, cursor_style));
        let after: String = chars[cursor + 1..].iter().collect();
        if !after.is_empty() {
            spans.push(Span::styled(after, base));
        }
    } else {
        // Caret at end of text: a block over a trailing space.
        spans.push(Span::styled(" ".to_string(), cursor_style));
    }
    spans
}

/// Horizontally centred sub-rect of `area`, full height.
pub fn centered_horizontal(area: Rect, percent_x: u16) -> Rect {
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(area)[1]
}

/// Centred rectangle occupying the given percentage of `area`.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}
