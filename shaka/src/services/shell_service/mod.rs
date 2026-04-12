use std::time::Duration;

use renge::Service;
use tokio::task;

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
        let (_id, mut shell_sender, shell_receiver) =
            multiplexer.spawn_separated(&asura::Config::default());

        let (content_sender, mut content_receiver) = tokio::sync::mpsc::channel(10);
        let task = task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
                match shell_receiver.try_recv_event() {
                    Ok(event) => match event {
                        asura::Event::Updated => {
                            let str: String = shell_receiver
                                .read_contents()
                                .acquire_contents()
                                .iter()
                                .map(|c| c.code)
                                .collect();
                            content_sender.send(str).await.unwrap();
                        }
                        asura::Event::Exit => continue,
                        asura::Event::Others => continue,
                    },
                    Err(_) => continue,
                }
            }
        });

        loop {
            tokio::select! {
                Some(action) = self.content_receiver.recv() => {
                    let Action::Input(str) = action else { continue };
                    shell_sender.send_input(&str);
                },
                Some(str) = content_receiver.recv() => {
                    self.content_sender.send(str).await.unwrap();
                },
                _ = &mut cancellation_token => break,
                else => {println!("else")}
            }
        }

        task.await.unwrap();
    }
}
