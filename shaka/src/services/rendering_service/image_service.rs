use renge::ParametricService;

use crate::services::rendering_service::{DescriptorSetOperationRequest, ImageData, ImageFormat};

#[allow(unused)]
pub struct ImageServiceParams {
    pub gfx_receiver: tokio::sync::mpsc::Sender<DescriptorSetOperationRequest>,
}

#[allow(unused)]
pub struct ImageService {}

impl ParametricService for ImageService {
    type Params = ImageServiceParams;

    async fn serve(self, params: Self::Params, _cancellation_token: renge::CancellationToken) {
        // TODO: ここで画像ファイルをロードする
        // 適当に 16x16 のグラデーションを画像を生成
        let width = 16;
        let height = 16;
        let mut data = vec![0f32; width * height * 3/*RGB*/];
        for y in 0..height {
            for x in 0..width {
                let i = (x + y * width) * 3;
                data[i] = x as f32 / width as f32;
                data[i + 1] = y as f32 / height as f32;
                data[i + 2] = 0f32;
            }
        }

        let (request, receiver) = DescriptorSetOperationRequest::new();
        params.gfx_receiver.send(request).await.unwrap();

        let mut handler = receiver.await.unwrap();

        handler.create_descriptor_sets(1);

        let image_data = ImageData {
            data,
            width: 8,
            height: 1,
            raw: 8,
            format: ImageFormat::R32g32b32Float,
        };
        handler.update_descriptor_set(0, image_data);
        handler.finish();
    }
}
