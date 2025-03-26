use std::time::Duration;

use futures::FutureExt;
use tracing::instrument;

pub struct PollingEventService {
    exit_receiver: tokio::sync::oneshot::Receiver<()>,

    senders: Vec<tokio::sync::mpsc::Sender<()>>,
}

impl PollingEventService {
    pub fn new(exit_receiver: tokio::sync::oneshot::Receiver<()>) -> Self {
        Self {
            exit_receiver,
            senders: Vec::default(),
        }
    }

    #[instrument]
    pub async fn serve(&mut self) {
        loop {
            // 一定のタイミングでポーリング
            let f = futures::future::poll_fn(|c| self.exit_receiver.poll_unpin(c));
            if let Ok(_) = tokio::time::timeout(Duration::from_millis(30), f).await {
                // 終了要求がきたのでループを抜ける
                break;
            }

            let fugures = self.senders.iter().map(|sender| sender.send(()));
            futures::future::join_all(fugures).await;
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<()> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        self.senders.push(sender);
        receiver
    }
}

impl std::fmt::Debug for PollingEventService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PollingEventService")?;
        std::fmt::Result::Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .build()
            .unwrap();

        let (sender, receiver) = tokio::sync::oneshot::channel();
        let mut service = PollingEventService::new(receiver);

        let task = runtime.spawn(async move {
            service.serve().await;
        });

        sender.send(()).unwrap();
        runtime.block_on(async {
            task.await.unwrap();
        });
    }
}
