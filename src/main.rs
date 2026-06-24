//! Linkly TUI — a terminal UI for managing Linkly short links.

mod api;
mod app;
mod config;
mod forms;
mod qr;
mod ui;

use anyhow::Result;
use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc;

use app::{App, AsyncMsg};

#[tokio::main]
async fn main() -> Result<()> {
    // `ratatui::init` enables raw mode + the alternate screen and installs a
    // panic hook that restores the terminal, so a crash never leaves a broken
    // shell. `ratatui::restore` undoes it on normal exit.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal).await;
    ratatui::restore();
    result
}

async fn run(terminal: &mut DefaultTerminal) -> Result<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<AsyncMsg>();
    let mut app = App::new(tx);
    let mut events = EventStream::new();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        tokio::select! {
            maybe_event = events.next() => match maybe_event {
                Some(Ok(Event::Key(key))) if key.kind == KeyEventKind::Press => {
                    app.on_event(Event::Key(key));
                }
                Some(Ok(_)) => {} // resize/mouse/etc. — redraw on next loop
                Some(Err(_)) | None => app.should_quit = true,
            },
            Some(msg) = rx.recv() => app.on_async(msg),
        }
    }

    Ok(())
}
