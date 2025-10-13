use std::time::Duration;

use tokio::time::Instant;
use winit::event::KeyEvent;

use crate::shell_service::SpawnRequest;

pub struct Input {
    pub id: asura::ShellId,
}

pub struct InputHandlingService {
    input_receiver: tokio::sync::mpsc::Receiver<KeyEvent>,
    swawn_request_sender: tokio::sync::mpsc::Sender<SpawnRequest>,
    input_sender: tokio::sync::mpsc::Sender<Input>,
    id: Option<asura::ShellId>,
}

impl InputHandlingService {
    pub fn new(
        input_receiver: tokio::sync::mpsc::Receiver<KeyEvent>,
    ) -> (
        Self,
        tokio::sync::mpsc::Receiver<SpawnRequest>,
        tokio::sync::mpsc::Receiver<Input>,
    ) {
        let (swawn_request_sender, spawn_request_receiver) = tokio::sync::mpsc::channel(8);
        let (input_sender, receiver) = tokio::sync::mpsc::channel(8);

        (
            Self {
                input_receiver,
                swawn_request_sender,
                input_sender,
                id: None,
            },
            spawn_request_receiver,
            receiver,
        )
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
            Some(key_event) = self.input_receiver.recv() => {},
            _ = &mut cancellation_token => break,
            else => {},
            }
        }
    }

    async fn request_spawn(&mut self) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let request = SpawnRequest {
            config: asura::Config::default(),
            ack: sender,
        };
        self.swawn_request_sender
            .send(request)
            .await
            .unwrap_or_default();

        let id = receiver.await.unwrap();
        self.id = Some(id);
    }
}

impl renge::Service for InputHandlingService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
