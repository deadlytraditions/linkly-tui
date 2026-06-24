//! Rendering. `draw` routes to the active screen's renderer.

mod auth;
mod create;
mod detail;
mod list;
mod workspace;

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Screen};

/// Shared brand banner, rendered on the startup screens.
pub const BANNER: &[&str] = &[
    r"  ██╗     ██╗███╗   ██╗██╗  ██╗██╗  ██╗   ██╗",
    r"  ██║     ██║████╗  ██║██║ ██╔╝██║  ╚██╗ ██╔╝",
    r"  ██║     ██║██╔██╗ ██║█████╔╝ ██║   ╚████╔╝ ",
    r"  ██║     ██║██║╚██╗██║██╔═██╗ ██║    ╚██╔╝  ",
    r"  ███████╗██║██║ ╚████║██║  ██╗███████╗██║   ",
    r"  ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝   ",
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
    }
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
