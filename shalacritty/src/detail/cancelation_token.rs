use std::future::Future;

use pin_project::pin_project;

pub struct CancelRequest {
    sender: tokio::sync::oneshot::Sender<()>,
}

impl CancelRequest {
    pub fn send(self) -> Result<(), ()> {
        let Ok(_) = self.sender.send(()) else {
            return Err(());
        };

        Ok(())
    }
}

#[derive(Debug)]
#[pin_project]
pub struct CancellationToken {
    #[pin]
    receiver: tokio::sync::oneshot::Receiver<()>,
}

impl CancellationToken {
    pub fn new() -> (CancelRequest, CancellationToken) {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        (CancelRequest { sender }, Self { receiver })
    }
}

impl Future for CancellationToken {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.project().receiver.poll(cx) {
            std::task::Poll::Ready(_) => std::task::Poll::Ready(()),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
