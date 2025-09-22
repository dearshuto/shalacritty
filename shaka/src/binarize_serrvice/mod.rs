use crate::glyph_extract_service::ExtractionInfo;

pub struct BinarizeService {
    glyph_receiver: tokio::sync::mpsc::Receiver<ExtractionInfo>,
}

impl BinarizeService {
    pub fn new(glyph_receiver: tokio::sync::mpsc::Receiver<ExtractionInfo>) -> Self {
        Self { glyph_receiver }
    }

    pub async fn serve(mut self) {
        loop {
            tokio::select!(
            Some(glyph_info) = self.glyph_receiver.recv() => self.apply_glyph(glyph_info).await,
            else => break,
            )
        }
    }

    async fn apply_glyph(&mut self, info: ExtractionInfo) {
        for (char, rasterized_glyph) in info.glyphs {
            //
        }
    }
}
