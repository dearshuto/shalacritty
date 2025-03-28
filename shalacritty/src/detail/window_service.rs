use std::collections::HashMap;

use winit::window::{Window, WindowId};

use crate::app::WindowCreatedEventArgs;

pub struct WindowService {
    window_receiver: std::sync::mpsc::Receiver<WindowCreatedEventArgs>,
    polling_event_receiver: tokio::sync::mpsc::Receiver<()>,

    window_table: HashMap<WindowId, Window>,

    senders: Vec<tokio::sync::mpsc::Sender<WindowId>>,
}

impl WindowService {
    pub fn new(
        window_receiver: std::sync::mpsc::Receiver<WindowCreatedEventArgs>,
        polling_event_receiver: tokio::sync::mpsc::Receiver<()>,
    ) -> Self {
        Self {
            window_receiver,
            polling_event_receiver,
            window_table: HashMap::default(),
            senders: Vec::default(),
        }
    }

    pub async fn serve(mut self) {
        while let Some(_) = self.polling_event_receiver.recv().await {
            match self.window_receiver.try_recv() {
                Ok(args) => {
                    self.window_table.insert(args.id, args.window);
                    for sender in &self.senders {
                        sender.send(args.id).await.unwrap();
                    }
                }
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        for window in self.window_table.values_mut() {
                            window.request_redraw();
                        }
                        continue;
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => break,
                },
            };
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<WindowId> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.senders.push(sender);
        receiver
    }
}
