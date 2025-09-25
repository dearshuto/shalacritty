pub struct BinarizeService {}

impl BinarizeService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            else => break,
            )
        }
    }
}
