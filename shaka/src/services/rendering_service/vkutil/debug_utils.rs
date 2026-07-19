use std::borrow::Cow;

use ash::{vk::DebugUtilsMessengerEXT, *};

pub struct DebugUtils {
    instance: ash::ext::debug_utils::Instance,
    messanger: DebugUtilsMessengerEXT,
}

impl DebugUtils {
    pub fn new(entry: &ash::Entry, instance: &ash::Instance) -> Self {
        let debug_utils_loader = ext::debug_utils::Instance::new(&entry, &instance);
        let debug_info = vk::DebugUtilsMessengerCreateInfoEXT::default()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(Self::vulkan_debug_callback));
        let debug_utils =
            unsafe { debug_utils_loader.create_debug_utils_messenger(&debug_info, None) }.unwrap();

        Self {
            instance: debug_utils_loader,
            messanger: debug_utils,
        }
    }

    extern "system" fn vulkan_debug_callback(
        message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
        message_type: vk::DebugUtilsMessageTypeFlagsEXT,
        p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
        _user_data: *mut std::os::raw::c_void,
    ) -> vk::Bool32 {
        let callback_data = unsafe { *p_callback_data };
        let message_id_number = callback_data.message_id_number;

        let message_id_name = if callback_data.p_message_id_name.is_null() {
            Cow::from("")
        } else {
            unsafe { std::ffi::CStr::from_ptr(callback_data.p_message_id_name).to_string_lossy() }
        };

        let message = if callback_data.p_message.is_null() {
            Cow::from("")
        } else {
            unsafe { std::ffi::CStr::from_ptr(callback_data.p_message).to_string_lossy() }
        };

        println!(
            "{message_severity:?}:\n{message_type:?} [{message_id_name} ({message_id_number})] : {message}\n",
        );

        vk::FALSE
    }
}

impl Drop for DebugUtils {
    fn drop(&mut self) {
        unsafe {
            self.instance
                .destroy_debug_utils_messenger(self.messanger, None)
        };
    }
}
