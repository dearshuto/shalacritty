use ash::*;

pub struct ApplicationInfoFactory;

impl ApplicationInfoFactory {
    pub fn create() -> vk::ApplicationInfo<'static> {
        vk::ApplicationInfo::default()
            .application_name(c"shalacritty")
            .engine_name(c"shalacritty")
            .application_version(0)
            .engine_version(0)
            .api_version(vk::API_VERSION_1_3)
    }
}
