fn main() {
    let source_and_output_path_iter = [
        (
            include_str!("res/background.vs"),
            naga::ShaderStage::Vertex,
            "res/background.vs.spv",
        ),
        (
            include_str!("res/background.fs"),
            naga::ShaderStage::Fragment,
            "res/background.fs.spv",
        ),
        (
            include_str!("res/character.vs.glsl"),
            naga::ShaderStage::Vertex,
            "res/character.vs.spv",
        ),
        (
            include_str!("res/character.fs.glsl"),
            naga::ShaderStage::Fragment,
            "res/character.fs.spv",
        ),
    ];
    for (soruce, stage, output_path) in source_and_output_path_iter {
        let mut binary = convert_shader(soruce, stage);
        let pte = binary.as_mut_ptr() as *mut u8;

        unsafe {
            std::fs::write(
                output_path,
                std::ptr::slice_from_raw_parts_mut(pte, binary.len() * 4)
                    .as_ref()
                    .unwrap(),
            )
        }
        .unwrap();
    }
}

fn convert_shader(source: &str, stage: naga::ShaderStage) -> Vec<u32> {
    let mut front = naga::front::glsl::Frontend::default();
    let module = front
        .parse(&naga::front::glsl::Options::from(stage), source)
        .unwrap();

    let module_info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap();
    naga::back::spv::write_vec(
        &module,
        &module_info,
        &naga::back::spv::Options::default(),
        None,
    )
    .unwrap()
}
