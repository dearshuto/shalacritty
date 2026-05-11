use ash::*;
use renge::ParametricService;

use crate::services::rendering_service::DescriptorSetOperationHandle;

#[allow(unused)]
pub struct ImageServiceParams {
    pub gfx_receiver: tokio::sync::oneshot::Receiver<DescriptorSetOperationHandle>,
}

#[allow(unused)]
pub struct ImageService {}

impl ParametricService for ImageService {
    type Params = ImageServiceParams;

    async fn serve(self, params: Self::Params, _cancellation_token: renge::CancellationToken) {
        let mut handler = params.gfx_receiver.await.unwrap();

        handler.create_descriptor_sets(1);
        handler.update_descriptor_set(0, vk::ImageView::null());
        handler.finish();
    }
}
