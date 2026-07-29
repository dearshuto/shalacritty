use ash::{vk::MemoryRequirements2, *};
use std::collections::HashMap;

#[derive(Debug, Hash)]
struct MemoryPoolTypeKey;

#[derive(Debug, Default)]
pub struct AllocateInfo {
    flags: vk::MemoryPropertyFlags,
    size: vk::DeviceSize,
    memory_type_bits: u32,
}

impl AllocateInfo {
    pub fn with_flags(mut self, flags: vk::MemoryPropertyFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_size(mut self, size: vk::DeviceSize) -> Self {
        self.size = size;
        self
    }

    pub fn with_memory_type_bits(mut self, bit: u32) -> Self {
        self.memory_type_bits = bit;
        self
    }
}

pub struct AllocateResult {
    pub device_memory: vk::DeviceMemory,
    pub offset: vk::DeviceSize,
}

pub struct DeviceMemoryAllocator {
    device: ash::Device,
    memory_pool: Vec<vk::DeviceMemory>,
    memory_types: Vec<vk::MemoryType>,
}

impl DeviceMemoryAllocator {
    pub fn new(device: ash::Device, memory_types: &[vk::MemoryType]) -> Self {
        Self {
            device,
            memory_pool: Vec::default(),
            memory_types: memory_types.to_vec(),
        }
    }

    pub fn allocate(&mut self, info: AllocateInfo) -> Option<AllocateResult> {
        let search_result = self
            .memory_types
            .iter()
            .enumerate()
            .find(|(index, memory_type)| {
                let flags =
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT;
                (1 << index) & info.memory_type_bits != 0
                    && memory_type.property_flags & flags == flags
            })
            .map(|(index, _)| index as u32);
        let Some(search_index) = search_result else {
            return None;
        };

        Some(AllocateResult {
            device_memory: self.memory_pool[search_index as usize],
            offset: 0,
        })
    }
}

impl Drop for DeviceMemoryAllocator {
    fn drop(&mut self) {
        while let Some(device_memory) = self.memory_pool.pop() {
            unsafe {
                self.device.free_memory(device_memory, None);
            }
        }

        self.memory_types.clear();
    }
}
