use winit::{event::WindowEvent, window::WindowId};

pub enum Action {}

pub struct InputEventService {
    receiver: tokio::sync::mpsc::Receiver<(WindowId, WindowEvent)>,
    #[allow(unused)]
    sender: tokio::sync::mpsc::Sender<Action>,
}

impl InputEventService {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<(WindowId, WindowEvent)>,
        sender: tokio::sync::mpsc::Sender<Action>,
    ) -> Self {
        InputEventService { receiver, sender }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select! {
                Some((_window_id, event)) = self.receiver.recv() => self.handle_event(event).await,
                else => {}
            }
        }
    }

    async fn handle_event(&mut self, event: WindowEvent) {
        match event {
            WindowEvent::KeyboardInput {
                #[allow(unused)]
                device_id,
                event,
                #[allow(unused)]
                is_synthetic,
            } => {
                let Some(sender) = &self.key_event_sender else {
                    return;
                };

                sender.send(event).unwrap_or_default();
            }
            WindowEvent::RedrawRequested => {
                let Some(sender) = &self.redraw_request_sender else {
                    return;
                };

                let sender_cloned = sender.clone();
                tokio::spawn(async move { sender_cloned.send(()).await.unwrap() });
            }
            WindowEvent::CloseRequested => {}
            _ => {}
        }
    }
}
