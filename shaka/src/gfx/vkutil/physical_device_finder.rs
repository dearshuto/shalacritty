use ash::*;

pub struct DeviceCapability {
    pub physical_device: vk::PhysicalDevice,
    pub graphics_queue_index: usize,
    pub graphics_queue_count: u32,
    pub transfer_queue_index: usize,
}

pub fn search_device_capability(instance: &ash::Instance) -> DeviceCapability {
    unsafe { instance.enumerate_physical_devices() }
        .unwrap()
        .iter()
        .find_map(|physical_device| {
            let properties =
                unsafe { instance.get_physical_device_queue_family_properties(*physical_device) };

            let mut graphics_queue_count = None;
            let mut graphics_queue_index = None;
            let mut transfer_queue_index = None;
            for (index, property) in properties.into_iter().enumerate() {
                if graphics_queue_index.is_none() {
                    if property.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                        graphics_queue_count = Some(property.queue_count);
                        graphics_queue_index = Some(index);
                    }
                }

                if transfer_queue_index.is_none() {
                    // 転送用は専用キューがあるか
                    if property.queue_flags == vk::QueueFlags::TRANSFER {
                        transfer_queue_index = Some(index);
                        break;
                    }

                    // 次点で描画キューと干渉しないで使えるキューがあるか
                    if property.queue_flags.contains(vk::QueueFlags::TRANSFER)
                        && !property.queue_flags.contains(vk::QueueFlags::GRAPHICS)
                    {
                        transfer_queue_index = Some(index);
                        break;
                    }
                }
            }
            // Transfer 専用キューが見つからない場合、Graphics が Transfer 能力を内包しているので相乗りさせる
            if transfer_queue_index.is_none() {
                transfer_queue_index = graphics_queue_index;
            }

            return Some(DeviceCapability {
                physical_device: *physical_device,
                graphics_queue_index: graphics_queue_index.unwrap(),
                graphics_queue_count: graphics_queue_count.unwrap(),
                transfer_queue_index: transfer_queue_index.unwrap(),
            });
        })
        .unwrap()
}
