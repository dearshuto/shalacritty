use shaka_plugin_api::fs::File;

#[unsafe(no_mangle)]
pub extern "C" fn init() {
    let Ok(_file) = File::open("texture.png") else {
        return;
    };
}
