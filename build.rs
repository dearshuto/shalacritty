use std::{
    fs::File,
    io::{BufWriter, Write},
};

fn main() {
    let settings = [
        (
            include_str!("res/rect.vs"),
            "src/gfx/detail/rect.vs.wgsl",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/rect.fs"),
            "src/gfx/detail/rect.fs.wgsl",
            naga::ShaderStage::Fragment,
        ),
        (
            include_str!("res/char_rect.vs"),
            "src/gfx/char_rect.vs.wgsl",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/char_rect.fs"),
            "src/gfx/char_rect.fs.wgsl",
            naga::ShaderStage::Fragment,
        ),
        (
            include_str!("res/background.vs"),
            "src/gfx/detail/background.vs.wgsl",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/background.fs"),
            "src/gfx/detail/background.fs.wgsl",
            naga::ShaderStage::Fragment,
        ),
        (
            include_str!("res/copy_scan_buffer.vs"),
            "src/gfx/detail/copy_scan_buffer.vs.wgsl",
            naga::ShaderStage::Vertex,
        ),
        (
            include_str!("res/copy_scan_buffer.fs"),
            "src/gfx/detail/copy_scan_buffer.fs.wgsl",
            naga::ShaderStage::Fragment,
        ),
    ];

    for setting in settings {
        let shader_binary = convert_to_wgsl(setting.0, setting.2);
        let shader_binary_file = File::create(setting.1).unwrap();
        let mut f = BufWriter::new(shader_binary_file);
        f.write_all(shader_binary.as_bytes()).unwrap();
    }
}

#[allow(dead_code)]
fn convert_to_spv(source: &str, stage: naga::ShaderStage) -> Vec<u8> {
    let options = naga::front::glsl::Options::from(stage);
    let vertex_module = naga::front::glsl::Frontend::default()
        .parse(&options, source)
        .unwrap();

    // BLOCKS のバリデーションに失敗するがシェーダーに問題はないのでスキップ
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::EXPRESSIONS
            // | naga::valid::ValidationFlags::BLOCKS
            | naga::valid::ValidationFlags::CONTROL_FLOW_UNIFORMITY
            | naga::valid::ValidationFlags::STRUCT_LAYOUTS
            | naga::valid::ValidationFlags::CONSTANTS
            | naga::valid::ValidationFlags::BINDINGS,
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

fn convert_to_wgsl(source: &str, stage: naga::ShaderStage) -> String {
    let options = naga::front::glsl::Options::from(stage);
    let vertex_module = naga::front::glsl::Frontend::default()
        .parse(&options, source)
        .unwrap();

    // BLOCKS のバリデーションに失敗するがシェーダーに問題はないのでスキップ
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::EXPRESSIONS
            // | naga::valid::ValidationFlags::BLOCKS
            | naga::valid::ValidationFlags::CONTROL_FLOW_UNIFORMITY
            | naga::valid::ValidationFlags::STRUCT_LAYOUTS
            | naga::valid::ValidationFlags::CONSTANTS
            | naga::valid::ValidationFlags::BINDINGS,
        naga::valid::Capabilities::all(),
    )
    .validate(&vertex_module)
    .unwrap();

    naga::back::wgsl::write_string(&vertex_module, &info, naga::back::wgsl::WriterFlags::all())
        .unwrap()
}
