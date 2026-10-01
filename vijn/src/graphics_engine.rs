use ash::*;

use crate::vkutil;

pub struct TransferWorkerContext {
    device: ash::Device,
    // CommandPool の所有権のみを持つ (Queue は持たない！)
    command_pool: vk::CommandPool,
}

pub struct GraphicsEngine {
    device: ash::Device,
    transfer_queue_family_index: u32,
}

impl GraphicsEngine {
    pub fn new(instance: &ash::Instance, physical_device: vk::PhysicalDevice) -> Self {
        let device = unsafe {
            let create_info = vk::DeviceCreateInfo::default();
            instance
                .create_device(physical_device, &create_info, None)
                .unwrap()
        };

        let device_capability = vkutil::search_device_capability(&instance);

        Self {
            device,
            transfer_queue_family_index: device_capability.transfer_queue_index,
        }
    }

    pub fn create_transfer_worker(&self) -> TransferWorkerContext {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(self.transfer_queue_family_index)
            // ワーカースレッドで個別にリセットできるようにフラグを設定
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);

        let command_pool = unsafe { self.device.create_command_pool(&pool_info, None).unwrap() };

        // ワーカースレッド専用の文脈（Context）として所有権ごと手渡す
        TransferWorkerContext {
            device: self.device.clone(),
            command_pool,
        }
    }
}
