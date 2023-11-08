use std::{collections::HashMap, fs::File, io::Write};

fn main() {
    {
        let vertex_shader_binary =
            convert_to_spv(include_str!("res/rect.vs"), naga::ShaderStage::Vertex);
        let mut vertex_shader_binary_file = File::create("src/gfx/rect.vs.spv").unwrap();
        vertex_shader_binary_file
            .write_all(&vertex_shader_binary)
            .unwrap();

        let pixel_shader_binary =
            convert_to_spv(include_str!("res/rect.fs"), naga::ShaderStage::Fragment);
        let mut pixel_shader_binary_file = File::create("src/gfx/rect.fs.spv").unwrap();
        pixel_shader_binary_file
            .write_all(&pixel_shader_binary)
            .unwrap();
    }

    {
        // 文字の背景
        // バリエーション
        let mut variation: HashMap<String, String> = HashMap::default();
        variation.insert("BACKGROUND".to_string(), "1".to_string());

        let vertex_shader_binary = convert_to_spv_with_defines(
            include_str!("res/rect.vs"),
            naga::ShaderStage::Vertex,
            &variation,
        );
        let mut vertex_shader_binary_file =
            File::create("src/gfx/character_background.vs.spv").unwrap();
        vertex_shader_binary_file
            .write_all(&vertex_shader_binary)
            .unwrap();

        let pixel_shader_binary = convert_to_spv_with_defines(
            include_str!("res/rect.fs"),
            naga::ShaderStage::Fragment,
            &variation,
        );
        let mut pixel_shader_binary_file =
            File::create("src/gfx/character_background.fs.spv").unwrap();
        pixel_shader_binary_file
            .write_all(&pixel_shader_binary)
            .unwrap();
    }

    {
        let vertex_shader_binary =
            convert_to_spv(include_str!("res/background.vs"), naga::ShaderStage::Vertex);
        let mut vertex_shader_binary_file = File::create("src/gfx/background.vs.spv").unwrap();
        vertex_shader_binary_file
            .write_all(&vertex_shader_binary)
            .unwrap();

        let pixel_shader_binary = convert_to_spv(
            include_str!("res/background.fs"),
            naga::ShaderStage::Fragment,
        );
        let mut pixel_shader_binary_file = File::create("src/gfx/background.fs.spv").unwrap();
        pixel_shader_binary_file
            .write_all(&pixel_shader_binary)
            .unwrap();
    }
}

fn convert_to_spv(source: &str, stage: naga::ShaderStage) -> Vec<u8> {
    convert_to_spv_with_defines(source, stage, &HashMap::default())
}

fn convert_to_spv_with_defines(
    source: &str,
    stage: naga::ShaderStage,
    define_map: &HashMap<String, String>,
) -> Vec<u8> {
    let mut options = naga::front::glsl::Options::from(stage);
    for (key, value) in define_map {
        options.defines.insert(key.clone(), value.clone());
    }
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
