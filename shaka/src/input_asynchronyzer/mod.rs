use std::time::Duration;

use winit::event::KeyEvent;

pub struct InputAsynchronyzer {
    input_receiver: std::sync::mpsc::Receiver<KeyEvent>,
    input_senders: Vec<tokio::sync::mpsc::Sender<KeyEvent>>,
}

impl InputAsynchronyzer {
    pub fn new(input_receiver: std::sync::mpsc::Receiver<KeyEvent>) -> Self {
        Self {
            input_receiver,
            input_senders: Vec::default(),
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<KeyEvent> {
        let (sender, receiver) = tokio::sync::mpsc::channel(10);
        self.input_senders.push(sender);
        receiver
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
            _ = tokio::time::sleep(Duration::from_millis(10)) => self.poll_input().await,
            _ = &mut cancellation_token => break,
            else => {},
            }
        }
    }

    async fn poll_input(&mut self) {
        match self.input_receiver.try_recv() {
            Ok(key_event) => {
                for sender in &self.input_senders {
                    sender.send(key_event.clone()).await.unwrap()
                }
            }
            Err(_) => return,
        }
    }
}

impl renge::Service for InputAsynchronyzer {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
