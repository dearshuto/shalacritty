use wasmtime::Caller;

pub fn get_invalid_handle() -> i64 {
    0i64
}

pub fn read_file(mut caller: Caller<'_, u32>, ptr: i32, len: i32) -> i64 {
    0i64
}
