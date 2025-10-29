mod host_api;

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
            .func_wrap("host", "host_func", host_api::func)
            .unwrap();

        let mut store = wasmtime::Store::new(&engine, 4);
        let instance = linker.instantiate(&mut store, &module).unwrap();

        Self { instance, store }
    }

    pub fn execute(&mut self) {
        let hello = self
            .instance
            .get_typed_func::<(), ()>(&mut self.store, "hello")
            .unwrap();

        hello.call(&mut self.store, ()).unwrap();
    }
}
