use renge::Service;

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<String>,
    content_receiver: tokio::sync::mpsc::Receiver<String>,
}

impl ShellService {
    pub fn new(content_sender: tokio::sync::mpsc::Sender<String>) -> Self {
        let (_content_sender, content_receiver) = tokio::sync::mpsc::channel(1);
        Self {
            content_sender,
            content_receiver,
        }
    }
}

impl Service for ShellService {
    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        self.content_sender
            .send("AABBCC".to_string())
            .await
            .unwrap();

        loop {
            tokio::select! {
                Some(_args) = self.content_receiver.recv() => {},
                _ = &mut cancellation_token => break,
                else => {}
            }
        }
    }
}
