pub struct BinarizeService {}

impl BinarizeService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select!(
                _ = &mut cancellation_token => {
                    break;
                },
                else => {
                },
            )
        }
    }
}

impl renge::Service for BinarizeService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
