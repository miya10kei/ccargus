use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use ratatui::widgets::Paragraph;

mod action;
mod app;
mod components;
mod event;
mod tui;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut tui = tui::Tui::new()?;
    let mut events = event::EventHandler::new(4.0, 60.0);

    loop {
        let event = events.next().await?;
        match event {
            event::Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
            event::Event::Render => {
                tui.draw(|frame| {
                    frame
                        .render_widget(Paragraph::new("ccargus - Press 'q' to quit"), frame.area());
                })?;
            }
            event::Event::Resize(..) | event::Event::Tick | event::Event::Error => {}
        }
    }

    tui.exit()?;
    Ok(())
}
