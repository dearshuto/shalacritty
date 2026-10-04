use ash::*;

fn main() {
    let entry = ash::Entry::linked();
    let instance = {
        let application_info = vk::ApplicationInfo::default();

        let extension_names = [
            ash::ext::debug_utils::NAME.as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            khr::get_physical_device_properties2::NAME.as_ptr(),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            khr::portability_enumeration::NAME.as_ptr(),
        ];
        let create_flags = if cfg!(any(target_os = "macos", target_os = "ios")) {
            ash::vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
        } else {
            ash::vk::InstanceCreateFlags::default()
        };
        let layer_names = [c"VK_LAYER_KHRONOS_validation".as_ptr()];
        let create_info = vk::InstanceCreateInfo::default()
            .application_info(&application_info)
            .enabled_layer_names(&layer_names)
            .enabled_extension_names(&extension_names)
            .flags(create_flags);
        unsafe { entry.create_instance(&create_info, None).unwrap() }
    };

    let device_capability = vijn::vkutil::search_device_capability(&instance);
    let mut graphics_engine = vijn::GraphicsEngine::new(&instance, &device_capability);

    let factory = asura::DefaultFactory::default();
    let mut system = asura::TerminalSystem::new(factory);

    system.send_input("ls".as_bytes()).unwrap();

    let stdin = std::io::stdin();
    let mut buffer = String::new();
    while let Ok(_) = stdin.read_line(&mut buffer) {
        if buffer.trim().is_empty() {
            break;
        }
        system.send_input(buffer.as_bytes()).unwrap();
        buffer.clear();

        while let Ok(_) = system.receive_dirty_timeout(std::time::Duration::from_millis(100)) {
            let now = std::time::SystemTime::now();
            let image = image::RgbImage::new(640, 480);
            image.save(format!("{:?}.png", now)).unwrap();

            // for y in 0..24 {
            //     for x in 0..80 {
            //         let Some(cell) = system.cell(y, x) else {
            //             print!(" ");
            //             continue;
            //         };
            //         print!("{}", cell.ch);
            //     }
            //     println!();
            // }
        }
    }

    graphics_engine.cleanup(&instance);
    unsafe { instance.destroy_instance(None) };
}
