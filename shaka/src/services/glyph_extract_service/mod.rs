use std::{collections::HashMap, time::Duration};

use font_kit::{
    canvas::{Canvas, Format, RasterizationOptions},
    family_name::FamilyName,
    font::Font,
    hinting::HintingOptions,
    properties::Properties,
    source::SystemSource,
};
use pathfinder_geometry::{transform2d::Transform2F, vector::Vector2I};

#[derive(Debug, Hash, Eq, PartialEq)]
pub struct FontId {
    internal: u32,
}

impl Default for FontId {
    fn default() -> Self {
        Self { internal: 0 }
    }
}

#[derive(Debug, Default)]
pub struct Glyph {
    pub code: char,

    pub width: i32,
    pub height: i32,

    // RGBA
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub struct GlyphRequest {
    pub code: char,
    pub font_id: FontId,
    pub size: f32,
    pub response: tokio::sync::oneshot::Sender<Glyph>,
}

pub struct GlyphExtractService {
    request_receiver: std::sync::mpsc::Receiver<GlyphRequest>,

    font_table: HashMap<FontId, Font>,
}

impl GlyphExtractService {
    pub fn new(request_receiver: std::sync::mpsc::Receiver<GlyphRequest>) -> Self {
        let font = SystemSource::new()
            .select_best_match(&[FamilyName::Monospace], &Properties::default())
            .unwrap()
            .load()
            .unwrap();
        Self {
            request_receiver,
            font_table: HashMap::from([(FontId::default(), font)]),
        }
    }

    pub fn serve(mut self) {
        loop {
            match self
                .request_receiver
                .recv_timeout(Duration::from_micros(100))
            {
                Ok(request) => self.handle_request(request),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    }

    fn handle_request(&mut self, request: GlyphRequest) {
        let Some(font) = self.font_table.get(&request.font_id) else {
            return;
        };

        let Some(glyph_id) = font.glyph_for_char(request.code) else {
            return;
        };

        let rect = font
            .raster_bounds(
                glyph_id,
                request.size,
                Transform2F::default(),
                HintingOptions::None,
                RasterizationOptions::GrayscaleAa,
            )
            .unwrap();
        let mut canvas = Canvas::new(Vector2I::new(rect.width(), rect.height()), Format::A8);
        // 左上をぴったり合わせる
        let transform = Transform2F::from_translation(-rect.origin().to_f32());
        font.rasterize_glyph(
            &mut canvas,
            glyph_id,
            request.size,
            transform,
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa,
        )
        .unwrap();

        let glyph = Glyph {
            code: request.code,
            width: rect.width(),
            height: rect.height(),
            data: canvas.pixels,
        };
        request.response.send(glyph).unwrap();
    }
}

impl std::fmt::Debug for GlyphExtractService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("GlyphExtractService")?;
        std::fmt::Result::Ok(())
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {}
}
