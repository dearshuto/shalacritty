#[unsafe(no_mangle)]
pub extern "C" fn init() {
    let Ok(_file) = shaka_plugin_api::File::open("texture.png") else {
        return;
    };
}
