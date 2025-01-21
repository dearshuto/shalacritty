pub struct ShaderReflection {}

impl ShaderReflection {
    pub fn new() -> Self {
        Self {}
    }

    pub fn parse(&self, spv: &[u8]) -> Result<Layout, ()> {
        let Ok(module) =
            naga::front::spv::parse_u8_slice(spv, &naga::front::spv::Options::default())
        else {
            return Err(());
        };

        Ok(Self::parse_core(&module))
    }

    pub fn parse_u32<T>(&self, spv: T) -> Result<Layout, ()>
    where
        T: Iterator<Item = u32>,
    {
        let frontend: naga::front::spv::Frontend<_> =
            naga::front::spv::Frontend::new(spv, &naga::front::spv::Options::default());

        let Ok(module) = frontend.parse() else {
            return Err(());
        };

        Ok(Self::parse_core(&module))
    }

    fn parse_core(module: &naga::Module) -> Layout {
        let mut resources = Vec::default();

        // エントリーポイントは main で固定にしている
        // MEMO: 現状は GLSL 限定なので main しか受け付けないけど、外部から指定できるようにしたくなるかも
        let inputs =
            if let Some(entry_point) = module.entry_points.iter().find(|x| x.name == "main") {
                // 頂点アトリビュートはエントリーポイントの引数として取得できる
                entry_point
                    .function
                    .arguments
                    .iter()
                    .filter_map(|x| {
                        let Some(_name) = &x.name else {
                            return None;
                        };

                        let Some(binding) = &x.binding else {
                            return None;
                        };

                        let naga::Binding::Location { location, .. } = binding else {
                            return None;
                        };

                        Some(Input {
                            location: *location,
                        })
                    })
                    .collect()
            } else {
                Vec::default()
            };

        for (_handle, global_variable) in module.global_variables.iter() {
            match global_variable.space {
                naga::AddressSpace::Handle => {
                    // サンプラーとテクスチャーはここの分岐
                    let var_type = &module.types[global_variable.ty];
                    match var_type.inner {
                        // naga::TypeInner::Scalar(..) => todo!(),
                        // naga::TypeInner::Vector { size, scalar } => todo!(),
                        // naga::TypeInner::Matrix { columns, rows, scalar } => todo!(),
                        // naga::TypeInner::Atomic(scalar) => todo!(),
                        // naga::TypeInner::Pointer { base, space } => todo!(),
                        // naga::TypeInner::ValuePointer { size, scalar, space } => todo!(),
                        // naga::TypeInner::Array { base, size, stride } => todo!(),
                        // naga::TypeInner::Struct { members, span } => todo!(),
                        naga::TypeInner::Image { .. } => resources.push(ResourceType::Texture(
                            global_variable.binding.as_ref().unwrap().binding,
                        )),
                        naga::TypeInner::Sampler { .. } => resources.push(ResourceType::Sampler(
                            global_variable.binding.as_ref().unwrap().binding,
                        )),
                        // naga::TypeInner::AccelerationStructure => todo!(),
                        // naga::TypeInner::RayQuery => todo!(),
                        // naga::TypeInner::BindingArray { base, size } => todo!(),
                        _ => {}
                    }
                }
                naga::AddressSpace::Uniform => {
                    // 定数バッファー
                    resources.push(ResourceType::Uniform(
                        global_variable.binding.as_ref().unwrap().binding,
                    ));
                }
                naga::AddressSpace::Storage { .. } => {
                    // SSBO
                    resources.push(ResourceType::UnorderedAccessBuffer(
                        global_variable.binding.as_ref().unwrap().binding,
                    ))
                }
                _ => {}
            }
        }

        Layout { inputs, resources }
    }
}

#[derive(Debug, PartialEq)]
pub struct Input {
    location: u32,
}

#[derive(Debug, PartialEq)]
pub enum ResourceType {
    Uniform(u32),
    UnorderedAccessBuffer(u32),
    Sampler(u32),
    Texture(u32),
}

pub struct Layout {
    inputs: Vec<Input>,
    resources: Vec<ResourceType>,
}

impl Layout {
    pub fn inputs(&self) -> &[Input] {
        &self.inputs
    }

    pub fn resources(&self) -> &[ResourceType] {
        &self.resources
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use super::*;

    #[test]
    fn new() {
        let source = r#"
#version 450

layout (location = 0) in vec2 i_Position;
layout (location = 1) in vec2 i_Uv;

layout (binding = 0) uniform View {
    vec4 u_Transform[2];
};

layout (binding = 1) uniform texture2D u_Texture;
layout (binding = 2) uniform sampler u_Sampler;

layout(std430, binding = 3) readonly buffer DataBuffer
{
    int u_Data[];
};

void main()
{
    gl_Position = vec4(i_Position.x, i_Position.y, i_Uv.x, i_Uv.y);
}

"#;

        let mut frontend = naga::front::glsl::Frontend::default();
        let module = frontend
            .parse(
                &naga::front::glsl::Options {
                    stage: naga::ShaderStage::Vertex,
                    defines: HashMap::default(),
                },
                source,
            )
            .unwrap();

        let info = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap();
        let binary =
            naga::back::spv::write_vec(&module, &info, &naga::back::spv::Options::default(), None)
                .unwrap();

        let shader_reflection = ShaderReflection::new();
        let layout = shader_reflection.parse_u32(binary.into_iter()).unwrap();

        // 頂点バッファーへの入力変数
        assert_eq!(
            layout.inputs,
            vec![Input { location: 0 }, Input { location: 1 }]
        );

        // リソースたち
        assert_eq!(
            layout.resources,
            vec![
                ResourceType::Uniform(0),
                ResourceType::Texture(1),
                ResourceType::Sampler(2),
                ResourceType::UnorderedAccessBuffer(3)
            ]
        );
    }
}
