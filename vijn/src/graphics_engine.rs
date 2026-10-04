use ash::*;

use crate::vkutil::{self, DeviceCapability};

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
    pub fn new(instance: &ash::Instance, device_capability: &DeviceCapability) -> Self {
        let features = ash::vk::PhysicalDeviceFeatures::default().shader_clip_distance(true);
        let priorities = [1.0];
        let queue_infos = [
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(device_capability.graphics_queue_index as u32)
                .queue_priorities(&priorities),
            vk::DeviceQueueCreateInfo::default()
                .queue_family_index(device_capability.transfer_queue_index as u32)
                .queue_priorities(&priorities),
        ];
        let device_extension_names_raw = [
            ash::khr::storage_buffer_storage_class::NAME.as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            ash::khr::portability_subset::NAME.as_ptr(),
        ];
        let mut vulkan_11_features =
            vk::PhysicalDeviceVulkan11Features::default().shader_draw_parameters(true);
        let mut vulkan_12_features =
            vk::PhysicalDeviceVulkan12Features::default().timeline_semaphore(true);
        let mut vulkan_13_features = vk::PhysicalDeviceVulkan13Features::default()
            .dynamic_rendering(true)
            .synchronization2(true);

        let device_create_info = ash::vk::DeviceCreateInfo::default()
            // キューファミリーインデックスはユニークでないといけないので、
            // Transfer キューが Graphics キューに相乗りしてる場合はグラフィックスだけが設定されるようにする
            .queue_create_infos(
                if device_capability.graphics_queue_index != device_capability.transfer_queue_index
                {
                    &queue_infos
                } else {
                    std::slice::from_ref(
                        &queue_infos[device_capability.graphics_queue_index as usize],
                    )
                },
            )
            .enabled_extension_names(&device_extension_names_raw)
            .enabled_features(&features)
            .push_next(&mut vulkan_11_features)
            .push_next(&mut vulkan_12_features)
            .push_next(&mut vulkan_13_features);
        ash::vk::DeviceCreateFlags::default();

        let device = unsafe {
            instance
                .create_device(device_capability.physical_device, &device_create_info, None)
                .unwrap()
        };

        let device_capability = vkutil::search_device_capability(&instance);

        Self {
            device,
            transfer_queue_family_index: device_capability.transfer_queue_index,
        }
    }

    pub fn cleanup(&mut self, instance: &ash::Instance) {
        unsafe {
            self.device.destroy_device(None);
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
