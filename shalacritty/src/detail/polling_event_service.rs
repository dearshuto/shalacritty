use std::time::Duration;

pub struct PollingEventService {
    receiver: std::sync::mpsc::Receiver<()>,

    senders: Vec<tokio::sync::mpsc::Sender<()>>,
}

impl PollingEventService {
    pub fn new(receiver: std::sync::mpsc::Receiver<()>) -> Self {
        Self {
            receiver,
            senders: Vec::default(),
        }
    }

    pub async fn serve(&mut self) {
        loop {
            // 一定のタイミングでポーリング
            tokio::time::sleep(Duration::from_millis(30)).await;

            match self.receiver.try_recv() {
                Ok(_) => break,
                Err(error) => match error {
                    std::sync::mpsc::TryRecvError::Empty => {
                        // 続行なので通知
                        let fugures = self.senders.iter().map(|sender| sender.send(()));
                        futures::future::join_all(fugures).await;

                        continue;
                    }
                    std::sync::mpsc::TryRecvError::Disconnected => break,
                },
            }
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<()> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        self.senders.push(sender);
        receiver
    }
}
