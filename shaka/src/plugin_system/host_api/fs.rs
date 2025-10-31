use wasmtime::Caller;

pub fn get_invalid_handle() -> i64 {
    0i64
}

pub fn read_file(_caller: Caller<'_, u32>, _ptr: i32, _len: i32) -> i64 {
    0i64
}
