use ash::vk::Handle;

pub struct GraphicsContext;

impl GraphicsContext {
    pub fn draw(
        command_buffer: ash::vk::CommandBuffer,
        vertex_count: i32,
        instance_count: i32,
        first_vertex: i32,
        first_instance: i32,
    ) {
        unsafe {
            detail::draw(
                command_buffer.as_raw() as i64,
                vertex_count,
                instance_count,
                first_vertex,
                first_instance,
            )
        }
    }
}

mod detail {
    unsafe extern "C" {
        pub fn draw(
            command_buffer_handle: i64,
            vertex_count: i32,
            instance_count: i32,
            first_vertex: i32,
            first_instance: i32,
        );
    }
}
