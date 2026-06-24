//! Create-link form screen, with an advanced-fields toggle and a domain picker.

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;
use crate::forms::{CreateForm, Field};
use crate::ui::{centered_rect, panel, status_bar, theme, with_status_bar};

/// Width the field labels are padded to, so all values line up.
const LABEL_WIDTH: usize = 16;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let (main, status) = with_status_bar(frame.area());

    let fields = app.create_form.fields();

    // Index in `fields` where the advanced section begins (for a divider), if shown.
    let advanced_start = if app.create_form.show_advanced {
        Some(9) // core fields count, see CreateForm::fields
    } else {
        None
    };

    let mut items: Vec<ListItem> = Vec::new();
    let mut focus_offset = 0; // dividers shift the visual selection index
    for (i, &f) in fields.iter().enumerate() {
        if advanced_start == Some(i) {
            items.push(divider());
            if i <= app.create_form.focus {
                focus_offset += 1;
            }
        }
        if matches!(f, Field::Submit) {
            items.push(blank());
            if i <= app.create_form.focus {
                focus_offset += 1;
            }
        }
        items.push(field_item(&app.create_form, f, i == app.create_form.focus));
    }

    let title = if app.create_form.show_advanced {
        "Create link · advanced"
    } else {
        "Create link · core"
    };

    let list = List::new(items).block(panel(title)).highlight_style(
        Style::default()
            .bg(theme::SELECT_BG)
            .add_modifier(Modifier::BOLD),
    );

    let mut state = ListState::default();
    state.select(Some(app.create_form.focus + focus_offset));
    frame.render_stateful_widget(list, main, &mut state);

    status_bar(
        frame,
        status,
        app,
        "Tab move · Space toggle · ^A advanced · Enter save (on Submit) · Esc cancel",
    );

    if app.create_form.domain_selector.is_some() {
        render_domain_selector(frame, app);
    }
}

fn divider<'a>() -> ListItem<'a> {
    ListItem::new(Line::from(Span::styled(
        "── advanced ──────────────────────────",
        Style::default().fg(theme::BORDER),
    )))
}

fn blank<'a>() -> ListItem<'a> {
    ListItem::new(Line::from(""))
}

fn field_item<'a>(form: &CreateForm, f: Field, focused: bool) -> ListItem<'a> {
    if matches!(f, Field::Submit) {
        let style = Style::default()
            .fg(theme::OK)
            .add_modifier(Modifier::BOLD);
        return ListItem::new(Line::from(Span::styled("  ▶ Submit — create link", style)));
    }

    let label = CreateForm::label(f);
    let pointer = if focused { "▍ " } else { "  " };

    let value_span = match f {
        Field::Domain => Span::styled(
            form.domain_display(),
            Style::default().fg(theme::ACCENT),
        ),
        _ if form.bool_value(f).is_some() => {
            let on = form.bool_value(f).unwrap();
            if on {
                Span::styled("◉ on", Style::default().fg(theme::OK))
            } else {
                Span::styled("◯ off", Style::default().fg(theme::MUTED))
            }
        }
        _ => {
            let mut v = form.input(f).map(|i| i.value().to_string()).unwrap_or_default();
            if focused {
                v.push('▏');
            }
            let style = if v.is_empty() {
                Style::default().fg(theme::MUTED)
            } else {
                Style::default().fg(Color::White)
            };
            Span::styled(v, style)
        }
    };

    ListItem::new(Line::from(vec![
        Span::styled(pointer, Style::default().fg(theme::ACCENT)),
        Span::styled(
            format!("{label:<LABEL_WIDTH$}"),
            Style::default().fg(theme::ACCENT_DIM),
        ),
        value_span,
    ]))
}

fn render_domain_selector(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(50, 50, frame.area());
    frame.render_widget(Clear, area);

    let selector = app.create_form.domain_selector.as_mut().unwrap();
    let items: Vec<ListItem> = selector
        .options
        .iter()
        .map(|opt| match opt {
            None => ListItem::new(Span::styled(
                "(default domain)",
                Style::default().fg(theme::MUTED),
            )),
            Some(name) => ListItem::new(Span::styled(name.clone(), Style::default().fg(Color::White))),
        })
        .collect();

    let list = List::new(items)
        .block(panel("Select domain"))
        .highlight_style(
            Style::default()
                .bg(theme::SELECT_BG)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▍ ");

    frame.render_stateful_widget(list, area, &mut selector.state);
}
