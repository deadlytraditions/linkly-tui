//! Startup credential prompt with a banner. The API key is rendered masked.

use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::ui::{centered_rect, theme};

const BANNER: &[&str] = &[
    r"  ██╗     ██╗███╗   ██╗██╗  ██╗██╗  ██╗   ██╗",
    r"  ██║     ██║████╗  ██║██║ ██╔╝██║  ╚██╗ ██╔╝",
    r"  ██║     ██║██╔██╗ ██║█████╔╝ ██║   ╚████╔╝ ",
    r"  ██║     ██║██║╚██╗██║██╔═██╗ ██║    ╚██╔╝  ",
    r"  ███████╗██║██║ ╚████║██║  ██╗███████╗██║   ",
    r"  ╚══════╝╚═╝╚═╝  ╚═══╝╚═╝  ╚═╝╚══════╝╚═╝   ",
];

pub fn draw(frame: &mut Frame, app: &App) {
    let area = centered_rect(64, 84, frame.area());

    let rows = Layout::vertical([
        Constraint::Length(BANNER.len() as u16), // banner
        Constraint::Length(2),                   // subtitle
        Constraint::Length(8),                   // form box
        Constraint::Length(1),                   // footer
    ])
    .flex(ratatui::layout::Flex::Center)
    .split(area);

    // Banner.
    let banner: Vec<Line> = BANNER
        .iter()
        .map(|l| Line::from(Span::styled(*l, Style::default().fg(theme::ACCENT))))
        .collect();
    frame.render_widget(
        Paragraph::new(banner).alignment(Alignment::Center),
        rows[0],
    );

    // Subtitle.
    frame.render_widget(
        Paragraph::new(Span::styled(
            "terminal client · sign in to continue",
            Style::default()
                .fg(theme::MUTED)
                .add_modifier(Modifier::ITALIC),
        ))
        .alignment(Alignment::Center),
        rows[1],
    );

    // Form box.
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BORDER))
        .title(Span::styled(
            " credentials ",
            Style::default().fg(theme::ACCENT),
        ));
    let inner = block.inner(rows[2]);
    frame.render_widget(block, rows[2]);

    let form = Layout::vertical([
        Constraint::Length(2), // api key
        Constraint::Length(2), // workspace id
        Constraint::Length(1), // error
    ])
    .horizontal_margin(2)
    .split(inner);

    let key_len = app.auth.api_key.value().chars().count();
    field(
        frame,
        form[0],
        "API key",
        &"•".repeat(key_len),
        app.auth.focus == 0,
    );
    field(
        frame,
        form[1],
        "Workspace ID",
        app.auth.workspace_id.value(),
        app.auth.focus == 1,
    );

    if let Some(e) = &app.auth.error {
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("⚠ {e}"),
                Style::default().fg(theme::ERROR),
            )),
            form[2],
        );
    }

    // Footer.
    frame.render_widget(
        Paragraph::new(Span::styled(
            "Tab switch field   ·   Enter continue   ·   Esc quit",
            Style::default().fg(theme::MUTED),
        ))
        .alignment(Alignment::Center),
        rows[3],
    );
}

fn field(frame: &mut Frame, area: Rect, label: &str, value: &str, focused: bool) {
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
    let cursor = if focused { "▏" } else { "" };
    let lines = vec![
        Line::from(vec![
            Span::styled(marker, Style::default().fg(theme::ACCENT)),
            Span::styled(label.to_string(), label_style),
        ]),
        Line::from(vec![
            Span::raw("  "),
            Span::styled(value.to_string(), value_style),
            Span::styled(cursor, Style::default().fg(theme::ACCENT)),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), area);
}
