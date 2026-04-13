use renge::ParametricService;
use tokio_stream::StreamExt;
use winit::{event::KeyEvent, platform::modifier_supplement::KeyEventExtModifierSupplement};

use crate::services::EventKind;

#[derive(Debug, PartialEq)]
pub enum Action {
    Input(String),
}

pub struct InputEventService {
    sender: tokio::sync::mpsc::Sender<Action>,
}

impl InputEventService {
    pub fn new(sender: tokio::sync::mpsc::Sender<Action>) -> Self {
        InputEventService { sender }
    }

    async fn serve(mut self, receiver: tokio::sync::broadcast::Receiver<EventKind>) {
        let mut stream =
            tokio_stream::wrappers::BroadcastStream::new(receiver).filter_map(|event_kind| {
                let Ok(event) = event_kind else {
                    return None;
                };

                let EventKind::KeyboardInput { event, .. } = event else {
                    return None;
                };

                Some(event)
            });

        loop {
            while let Some(event) = stream.next().await {
                self.handle_key_event(event).await;
            }
        }
    }

    async fn handle_key_event(&mut self, event: KeyEvent) {
        if event.state.is_pressed() {
            if let Some(text) = event.text_with_all_modifiers() {
                if !text.is_empty() {
                    let _ = self.sender.send(Action::Input(text.to_string())).await;
                }
            }
        }
    }
}

impl ParametricService for InputEventService {
    type Params = tokio::sync::broadcast::Receiver<EventKind>;

    async fn serve(self, receiver: Self::Params, _cancellation_token: renge::CancellationToken) {
        self.serve(receiver).await;
    }
}
