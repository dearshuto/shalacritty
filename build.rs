use std::{fs::File, io::Write};

fn main() {
    let settings = [
        (
            include_str!("res/rect.vs"),
            "src/gfx/detail/rect.vs.spv",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/rect.fs"),
            "src/gfx/detail/rect.fs.spv",
            naga::ShaderStage::Fragment,
        ),
        (
            include_str!("res/char_rect.vs"),
            "src/gfx/char_rect.vs.spv",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/char_rect.fs"),
            "src/gfx/char_rect.fs.spv",
            naga::ShaderStage::Fragment,
        ),
        (
            include_str!("res/background.vs"),
            "src/gfx/background.vs.spv",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/background.fs"),
            "src/gfx/background.fs.spv",
            naga::ShaderStage::Fragment,
        ),
    ];

    for setting in settings {
        let shader_binary = convert_to_spv(setting.0, setting.2);
        let mut shader_binary_file = File::create(setting.1).unwrap();
        shader_binary_file.write_all(&shader_binary).unwrap();
    }
}

fn convert_to_spv(source: &str, stage: naga::ShaderStage) -> Vec<u8> {
    let options = naga::front::glsl::Options::from(stage);
    let vertex_module = naga::front::glsl::Frontend::default()
        .parse(&options, source)
        .unwrap();

    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&vertex_module)
    .unwrap();
    let options = naga::back::spv::Options::default();
    let mut data = naga::back::spv::write_vec(&vertex_module, &info, &options, None).unwrap();

    let ratio = std::mem::size_of::<u32>() / std::mem::size_of::<u8>();
    let length = data.len() * ratio;
    let capacity = data.capacity() * ratio;
    let ptr = data.as_mut_ptr() as *mut u8;
    unsafe {
        let u8_data: Vec<u8> = Vec::from_raw_parts(ptr, length, capacity).clone();

        // 元データが 2 重に破棄されないように、元データを破棄しないようにする
        std::mem::forget(data);

        u8_data
    }
}
