use std::path::Path;

pub struct PluginSystem {
    instance: wasmtime::Instance,
    store: wasmtime::Store<u32>,
}

impl PluginSystem {
    pub fn new() -> Self {
        let engine = wasmtime::Engine::default();
        let wat = r#"
        (module
            (import "host" "host_func" (func $host_hello (param i32)))

            (func (export "hello")
                i32.const 3
                call $host_hello)
        )
    "#;
        let module = wasmtime::Module::new(&engine, wat).unwrap();

        let mut linker = wasmtime::Linker::new(&engine);
        linker
            .func_wrap(
                "host",
                "host_func",
                |caller: wasmtime::Caller<'_, u32>, param: i32| {
                    println!("Got {} from WebAssembly", param);
                    println!("my host state is: {}", caller.data());
                },
            )
            .unwrap();

        // All wasm objects operate within the context of a "store". Each
        // `Store` has a type parameter to store host-specific data, which in
        // this case we're using `4` for.
        let mut store = wasmtime::Store::new(&engine, 4);
        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self { instance, store }
    }

    pub fn execute(&mut self) {
        let hello = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "hello")
            .unwrap();

        // And finally we can call the wasm!
        hello.call(&mut self.store, ()).unwrap();
    }

    fn load_plugin<T>(path: T) -> Result<i32, ()>
    where
        T: AsRef<Path>,
    {
        let _ = std::env::current_exe();
        let _ = std::fs::read_dir(path.as_ref());

        let Ok(is_exist) = std::fs::exists(path.as_ref()) else {
            return Err(());
        };

        if !is_exist {
            return Err(());
        }

        let str = libloading::library_filename(path.as_ref());
        let lib = unsafe { libloading::Library::new(str).unwrap() };
        todo!()
    }
}
