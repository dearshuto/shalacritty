fn main() {
    let mut binary = convert_shader(
        include_str!("res/hello_triangle.vs.glsl"),
        naga::ShaderStage::Vertex,
    );
    let pte = binary.as_mut_ptr() as *mut u8;

    unsafe {
        std::fs::write(
            "res/hello_triangle.vs.spv",
            std::ptr::slice_from_raw_parts_mut(pte, binary.len() * 4)
                .as_ref()
                .unwrap(),
        )
    }
    .unwrap();

    let mut binary = convert_shader(
        include_str!("res/hello_triangle.fs.glsl"),
        naga::ShaderStage::Fragment,
    );
    let pte = binary.as_mut_ptr() as *mut u8;

    unsafe {
        std::fs::write(
            "res/hello_triangle.fs.spv",
            std::ptr::slice_from_raw_parts_mut(pte, binary.len() * 4)
                .as_ref()
                .unwrap(),
        )
    }
    .unwrap();
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
