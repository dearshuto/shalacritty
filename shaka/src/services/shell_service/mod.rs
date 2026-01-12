use std::time::Duration;

use renge::Service;
use winit::{
    event::{DeviceId, KeyEvent},
    platform::modifier_supplement::KeyEventExtModifierSupplement,
    window::WindowId,
};

use crate::services::window_event_asynchronizer::AsyncWindowEvent;

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<String>,
    window_event_receiver: tokio::sync::mpsc::Receiver<(WindowId, AsyncWindowEvent)>,
    shell_controller: Option<asura::ShellController>,
}

impl ShellService {
    pub fn new(
        content_sender: tokio::sync::mpsc::Sender<String>,
        window_event_receiver: tokio::sync::mpsc::Receiver<(WindowId, AsyncWindowEvent)>,
    ) -> Self {
        Self {
            content_sender,
            window_event_receiver,
            shell_controller: None,
        }
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut multiplexer = asura::Multiplexer::new();
        let (_id, shell_controller) = multiplexer.spawn(&asura::Config::default());

        self.shell_controller = Some(shell_controller);
        loop {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_millis(100)) =>  {
                    if let Some(content) = self.poll_contents() {
                        self.content_sender.send(content).await.unwrap();
                    }
                },
                Some((window_id, event)) = self.window_event_receiver.recv() => self.handle_window_event(window_id, event),
                _ = &mut cancellation_token => break,
                else => {}
            }
        }
    }

    fn poll_contents(&self) -> Option<String> {
        let Some(controller) = &self.shell_controller else {
            return None;
        };

        match controller.recv_event() {
            Ok(event) => match event {
                asura::Event::Updated => {
                    let content = controller
                        .read_contents()
                        .acquire_contents()
                        .iter()
                        .filter_map(|c| if c.code != ' ' { Some(c.code) } else { None })
                        .collect();
                    return Some(content);
                }
                asura::Event::Exit => return None,
                asura::Event::Others => return None,
            },
            Err(_) => return None,
        };
    }

    fn handle_window_event(&mut self, _window_id: WindowId, event: AsyncWindowEvent) {
        match event {
            AsyncWindowEvent::RedrawRequested => return,
            AsyncWindowEvent::Resized {
                #[allow(unused)]
                size,
            } => { /* TODO */ }
            AsyncWindowEvent::KeyboardInput { device_id, event } => {
                self.handle_keyboard_input(device_id, event)
            }
        }
    }

    fn handle_keyboard_input(&mut self, _device_id: DeviceId, event: KeyEvent) {
        let Some(text) = event.text_with_all_modifiers() else {
            return;
        };

        let Some(shell_controller) = &mut self.shell_controller else {
            return;
        };

        shell_controller.send_input(text);
    }
}

impl Service for ShellService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
