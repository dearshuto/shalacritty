use renge::ParametricService;

use crate::services::rendering_service::DescriptorSetOperationRequest;

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
        let data = [0u8; 8];

        let (request, receiver) = DescriptorSetOperationRequest::new();
        params.gfx_receiver.send(request).await.unwrap();

        let mut handler = receiver.await.unwrap();

        handler.create_descriptor_sets(1);
        handler.update_descriptor_set(0, &data);
        handler.finish();
    }
}
