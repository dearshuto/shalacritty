use std::ffi::c_void;

#[repr(C)]
pub struct CharacterData {
    pub transform0: [f32; 4],
    pub transform1: [f32; 4],
    pub fg_color: [f32; 4],
    pub uv01: [f32; 4],
}
pub struct BufferView {
    ptr: *mut c_void,
}

impl BufferView {
    pub fn new(ptr: *mut c_void) -> Self {
        Self { ptr }
    }

    pub fn vertex_buffer(&mut self) -> &mut [f32] {
        let ptr = self.ptr as *mut f32;
        unsafe { std::slice::from_raw_parts_mut(ptr, 16) }
    }

    pub fn index_buffer(&mut self) -> &mut [u16] {
        let ptr = unsafe { (self.ptr as *mut f32).add(16) } as *mut u16;
        unsafe { std::slice::from_raw_parts_mut(ptr, 16) }
    }

    pub fn character_data(&mut self) -> &mut [CharacterData] {
        let ptr = unsafe { self.ptr.byte_add(64 + 256) } as *mut CharacterData;
        unsafe { std::slice::from_raw_parts_mut(ptr, 1024) }
    }
}
