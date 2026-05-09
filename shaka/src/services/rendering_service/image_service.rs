use ash::*;
use renge::ParametricService;

#[allow(unused)]
pub struct GraphicsObject {
    pub device: ash::Device,
    pub layout: vk::DescriptorSetLayout,
    pub descriptor_pool: vk::DescriptorPool,
}

#[allow(unused)]
pub struct DescriptorSetNotificationArgs {
    pub descriptor_sets: [vk::DescriptorSet; 5],
}

#[allow(unused)]
pub struct ImageServiceParams {
    pub gfx_receiver: tokio::sync::oneshot::Receiver<GraphicsObject>,

    pub new_set_sender: tokio::sync::mpsc::Sender<DescriptorSetNotificationArgs>,
    pub old_set_sender: tokio::sync::mpsc::Sender<DescriptorSetNotificationArgs>,
}

#[allow(unused)]
pub struct ImageService {}

impl ParametricService for ImageService {
    type Params = ImageServiceParams;

    async fn serve(self, params: Self::Params, _cancellation_token: renge::CancellationToken) {
        let graphics_object = params.gfx_receiver.await.unwrap();
        let device = graphics_object.device;
        let descriptor_pool = graphics_object.descriptor_pool;
        let descriptor_set_layout = graphics_object.layout;

        let descriptor_sets = {
            let set_layouts = [descriptor_set_layout];
            let allocate_info = vk::DescriptorSetAllocateInfo::default()
                .descriptor_pool(descriptor_pool)
                .set_layouts(&set_layouts);
            unsafe { device.allocate_descriptor_sets(&allocate_info) }.unwrap()
        };
        let mut args = DescriptorSetNotificationArgs {
            descriptor_sets: Default::default(),
        };
        for (i, descriptor_set) in descriptor_sets.into_iter().enumerate() {
            args.descriptor_sets[i] = descriptor_set;
        }
        params.new_set_sender.send(args).await.unwrap();
    }
}
