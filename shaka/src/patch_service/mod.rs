use crate::glyph_extract_service::ExtractionInfo;

pub struct Patch {}

pub struct PatchReceiver {
    receiver: tokio::sync::mpsc::Receiver<Patch>,
}

impl PatchReceiver {
    pub async fn recv(&mut self) -> Option<Patch> {
        self.receiver.recv().await
    }
}

pub struct PatchService {}

impl PatchService {
    pub fn new(
        glyph_receiver: tokio::sync::mpsc::Receiver<ExtractionInfo>,
    ) -> (PatchReceiver, Self) {
        todo!()
    }
}

impl renge::Service for PatchService {
    async fn serve(self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                _ = &mut cancellation_token => {},
                else => {}
            }
        }
    }
}
