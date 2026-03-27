use renge::Service;

use crate::services::Action;

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<String>,
    content_receiver: tokio::sync::mpsc::Receiver<Action>,
}

impl ShellService {
    pub fn new(
        content_sender: tokio::sync::mpsc::Sender<String>,
        content_receiver: tokio::sync::mpsc::Receiver<Action>,
    ) -> Self {
        Self {
            content_sender,
            content_receiver,
        }
    }
}

impl Service for ShellService {
    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut multiplexer = asura::Multiplexer::new();
        let (_id, mut controller) = multiplexer.spawn(&asura::Config::default());
        loop {
            match controller.recv_event() {
                Ok(event) => match event {
                    asura::Event::Updated => break,
                    asura::Event::Exit => return,
                    asura::Event::Others => continue,
                },
                Err(_) => return,
            }
        }

        let content = controller
            .read_contents()
            .acquire_contents()
            .iter()
            .filter_map(|c| if c.code != ' ' { Some(c.code) } else { None })
            .take(8)
            .collect();
        self.content_sender.send(content).await.unwrap();

        loop {
            tokio::select! {
                Some(action) = self.content_receiver.recv() => {
                    let Action::Input(str) = action else { continue };
                    controller.send_input(&str);
                },
                _ = &mut cancellation_token => break,
                else => {}
            }
        }
    }
}
