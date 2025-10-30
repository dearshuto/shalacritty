mod host_api;

pub struct PluginSystem {
    instance: wasmtime::Instance,
    store: wasmtime::Store<u32>,
}

impl PluginSystem {
    pub fn new() -> Self {
        let engine = wasmtime::Engine::default();
        let wat =
            include_bytes!("../../../target/wasm32-unknown-unknown/debug/background_renderer.wasm");
        let module = wasmtime::Module::new(&engine, wat).unwrap();

        let mut linker = wasmtime::Linker::new(&engine);
        linker
            .func_wrap("host", "host_func", host_api::func)
            .unwrap();
        linker
            .func_wrap(
                "env",
                "get_invalid_handle",
                host_api::fs::get_invalid_handle,
            )
            .unwrap();
        linker
            .func_wrap("env", "read_file", host_api::fs::read_file)
            .unwrap();

        let mut store = wasmtime::Store::new(&engine, 4);
        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self { instance, store }
    }

    pub fn execute(&mut self) {
        let hello = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "init")
            .unwrap();

        hello.call(&mut self.store, ()).unwrap();
    }
}
