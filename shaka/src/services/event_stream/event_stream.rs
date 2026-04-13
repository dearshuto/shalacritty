use renge::Service;
use winit::event::WindowEvent;
use winit::window::WindowId;

#[derive(Debug, Clone)]
pub enum EventKind {
    KeyboardInput {
        #[allow(unused)]
        device_id: winit::event::DeviceId,
        event: winit::event::KeyEvent,
        #[allow(unused)]
        is_synthetic: bool,
    },
    Resized {
        #[allow(unused)]
        width: u32,
        #[allow(unused)]
        height: u32,
    },
}

pub struct StreamingEvent {
    #[allow(unused)]
    pub window_id: WindowId,
    pub window_event: WindowEvent,
}

pub struct EventStream {
    receiver: tokio::sync::mpsc::Receiver<StreamingEvent>,
    sender: tokio::sync::broadcast::Sender<EventKind>,
}

impl EventStream {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<StreamingEvent>,
        sender: tokio::sync::broadcast::Sender<EventKind>,
    ) -> Self {
        Self { receiver, sender }
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                Some(event) = self.receiver.recv() => self.handle_event(event).await,
                _ = &mut cancellation_token => break,
                else => break,
            }
        }
    }

    async fn handle_event(&mut self, event: StreamingEvent) {
        match event.window_event {
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                self.sender
                    .send(EventKind::KeyboardInput {
                        event,
                        device_id,
                        is_synthetic,
                    })
                    .unwrap();
            }
            WindowEvent::Resized(size) => {
                self.sender
                    .send(EventKind::Resized {
                        width: size.width,
                        height: size.height,
                    })
                    .unwrap();
            }
            _ => {}
        }
    }
}

impl Service for EventStream {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await
    }
}
