use std::time::Duration;

use winit::event::WindowEvent;

pub enum AsyncWindowEvent {
    RedrawRequested,
    Resized {
        size: winit::dpi::PhysicalSize<u32>,
    },
    KeyboardInput {
        device_id: winit::event::DeviceId,
        event: winit::event::KeyEvent,
    },
}

pub struct WindowEventAsynchronizer {
    sender: tokio::sync::mpsc::Sender<(winit::window::WindowId, AsyncWindowEvent)>,
    receiver: std::sync::mpsc::Receiver<(winit::window::WindowId, winit::event::WindowEvent)>,
}

impl WindowEventAsynchronizer {
    pub fn is_async_event(event: &winit::event::WindowEvent) -> bool {
        match event {
            // winit::event::WindowEvent::RedrawRequested => true,
            winit::event::WindowEvent::Resized(_) => true,
            winit::event::WindowEvent::KeyboardInput { .. } => true,
            _ => false,
        }
    }

    pub fn new(
        senders: tokio::sync::mpsc::Sender<(winit::window::WindowId, AsyncWindowEvent)>,
        receiver: std::sync::mpsc::Receiver<(winit::window::WindowId, winit::event::WindowEvent)>,
    ) -> Self {
        Self {
            sender: senders,
            receiver,
        }
    }

    pub async fn serve(mut self) {
        loop {
            match self.receiver.recv_timeout(Duration::from_millis(20)) {
                Ok((id, event)) => self.handle_event(id, event).await,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    async fn handle_event(
        &mut self,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if !Self::is_async_event(&event) {
            return;
        }

        match event {
            WindowEvent::Resized(size) => {
                self.sender
                    .send((window_id, AsyncWindowEvent::Resized { size }))
                    .await
                    .unwrap_or_default();
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                #[allow(unused)]
                is_synthetic,
            } => {
                self.sender
                    .send((
                        window_id,
                        AsyncWindowEvent::KeyboardInput { device_id, event },
                    ))
                    .await
                    .unwrap_or_default();
            }
            WindowEvent::RedrawRequested => {
                self.sender
                    .send((window_id, AsyncWindowEvent::RedrawRequested))
                    .await
                    .unwrap_or_default();
            }
            _ => {}
        }
    }
}
