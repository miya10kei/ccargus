use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{EventStream, KeyEvent, MouseEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Event {
    Error,
    Key(KeyEvent),
    Mouse(MouseEvent),
    Render,
    Resize(u16, u16),
    Tick,
}

pub struct EventHandler {
    rx: mpsc::UnboundedReceiver<Event>,
    #[allow(dead_code)]
    task: tokio::task::JoinHandle<()>,
}

impl EventHandler {
    pub fn new(tick_rate: f64, frame_rate: f64) -> Self {
        let tick_interval = Duration::from_secs_f64(1.0 / tick_rate);
        let render_interval = Duration::from_secs_f64(1.0 / frame_rate);
        let (tx, rx) = mpsc::unbounded_channel();

        let task = tokio::spawn(async move {
            let mut reader = EventStream::new();
            let mut tick_timer = tokio::time::interval(tick_interval);
            let mut render_timer = tokio::time::interval(render_interval);

            loop {
                let event = tokio::select! {
                    _ = tick_timer.tick() => Event::Tick,
                    _ = render_timer.tick() => Event::Render,
                    event = reader.next() => match event {
                        Some(Ok(crossterm::event::Event::Key(key))) => Event::Key(key),
                        Some(Ok(crossterm::event::Event::Mouse(mouse))) => Event::Mouse(mouse),
                        Some(Ok(crossterm::event::Event::Resize(cols, rows))) => Event::Resize(cols, rows),
                        Some(Err(_)) => Event::Error,
                        _ => continue,
                    },
                };

                if tx.send(event).is_err() {
                    break;
                }
            }
        });

        Self { rx, task }
    }

    pub async fn next(&mut self) -> Result<Event> {
        self.rx
            .recv()
            .await
            .ok_or_else(|| color_eyre::eyre::eyre!("Event channel closed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_handler_creation_does_not_panic() {
        let _handler = EventHandler::new(4.0, 60.0);
    }

    #[tokio::test]
    #[ignore = "requires a real terminal"]
    async fn event_handler_receives_tick() {
        let mut handler = EventHandler::new(4.0, 60.0);
        let event = tokio::time::timeout(Duration::from_secs(1), handler.next())
            .await
            .expect("timed out waiting for event")
            .expect("failed to receive event");

        assert!(matches!(event, Event::Tick | Event::Render));
    }
}
