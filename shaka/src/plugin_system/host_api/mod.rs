pub mod fs;
pub mod gfx;

pub fn func(caller: wasmtime::Caller<'_, u32>, param: i32) {
    println!("Got {} from WebAssembly", param);
    println!("my host state is: {}", caller.data());
}
