use ash::*;

pub enum DrawPass {
    Background,
    Character,
}

pub struct DescriptorSetUtils {}

impl DescriptorSetUtils {
    pub fn descriptor_set_layout_bindings(
        pass: DrawPass,
    ) -> &'static [vk::DescriptorSetLayoutBinding<'static>] {
        const BACKGROUND_BINDINGS: &[vk::DescriptorSetLayoutBinding<'static>] = &[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: std::marker::PhantomData,
            },
            vk::DescriptorSetLayoutBinding {
                binding: 2,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: std::marker::PhantomData,
            },
            vk::DescriptorSetLayoutBinding {
                binding: 3,
                descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::VERTEX,
                p_immutable_samplers: std::ptr::null(),
                _marker: std::marker::PhantomData,
            },
        ];

        const CHARACTER_BINDINGS: &[vk::DescriptorSetLayoutBinding<'static>] = &[
            vk::DescriptorSetLayoutBinding {
                binding: 0,
                descriptor_type: vk::DescriptorType::SAMPLER,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: std::marker::PhantomData,
            },
            vk::DescriptorSetLayoutBinding {
                binding: 1,
                descriptor_type: vk::DescriptorType::SAMPLED_IMAGE,
                descriptor_count: 1,
                stage_flags: vk::ShaderStageFlags::FRAGMENT,
                p_immutable_samplers: std::ptr::null(),
                _marker: std::marker::PhantomData,
            },
        ];

        match pass {
            DrawPass::Background => BACKGROUND_BINDINGS,
            DrawPass::Character => CHARACTER_BINDINGS,
        }
    }
}
