pub struct File;

impl File {
    pub fn open(path: &str) -> Result<Self, ()> {
        let handle = unsafe { detail::read_file(path.as_ptr() as i32, path.len() as i32) };
        if handle == unsafe { detail::get_invalid_handle() } {
            return Err(());
        }

        Ok(Self {})
    }
}

mod detail {
    unsafe extern "C" {
        pub fn get_invalid_handle() -> i64;

        pub fn read_file(ptr: i32, len: i32) -> i64;
    }
}
