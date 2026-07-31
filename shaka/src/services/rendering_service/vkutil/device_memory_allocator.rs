use std::collections::HashMap;

use ash::*;

struct MemoryInfo {
    device_memory: vk::DeviceMemory,
    capacity: vk::DeviceSize,
    used: vk::DeviceSize,
}

#[derive(Debug, Default)]
pub struct AllocateInfo {
    pub memory_type_index: u32,
    pub size: vk::DeviceSize,
    pub alignlemt: vk::DeviceSize,
}

pub struct AllocateResult {
    pub device_memory: vk::DeviceMemory,
    pub offset: vk::DeviceSize,
}

pub struct DeviceMemoryAllocator {
    device: ash::Device,
    memory_pool: HashMap<u32, Vec<MemoryInfo>>,
}

impl DeviceMemoryAllocator {
    pub fn new(device: ash::Device) -> Self {
        Self {
            device,
            memory_pool: HashMap::default(),
        }
    }

    pub fn allocate(&mut self, info: AllocateInfo) -> Option<AllocateResult> {
        let memory_info: Option<MemoryInfo> = None;
        let Some(memory_info) = memory_info else {
            return None;
        };

        Some(AllocateResult {
            device_memory: memory_info.device_memory,
            offset: 0,
        })
    }
}

impl Drop for DeviceMemoryAllocator {
    fn drop(&mut self) {
        for memory_infos in self.memory_pool.values_mut() {
            while let Some(memory_info) = memory_infos.pop() {
                unsafe {
                    self.device.free_memory(memory_info.device_memory, None);
                }
            }
        }
    }
}
