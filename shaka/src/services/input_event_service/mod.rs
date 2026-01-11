use std::time::Duration;

use winit::{event::KeyEvent, platform::modifier_supplement::KeyEventExtModifierSupplement};

pub enum Action {}

pub struct InputEventService {
    receiver: std::sync::mpsc::Receiver<KeyEvent>,
    #[allow(unused)]
    sender: tokio::sync::mpsc::Sender<Action>,
}

impl InputEventService {
    pub fn new(
        receiver: std::sync::mpsc::Receiver<KeyEvent>,
        sender: tokio::sync::mpsc::Sender<Action>,
    ) -> Self {
        InputEventService { receiver, sender }
    }

    pub async fn serve(mut self) {
        loop {
            match self.receiver.recv_timeout(Duration::from_micros(100)) {
                Ok(request) => self.handle_key_event(request).await,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    async fn handle_key_event(&mut self, event: KeyEvent) {
        let Some(_text) = event.text_with_all_modifiers() else {
            return;
        };

        // TODO
    }
}
