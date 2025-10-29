pub struct File;

impl File {
    pub fn open(_path: &str) -> Result<Self, ()> {
        Err(())
    }
}

unsafe extern "C" {
    pub fn get_invalid_handle() -> i64;

    pub fn read_file(ptr: i32, len: i32) -> i64;
}
