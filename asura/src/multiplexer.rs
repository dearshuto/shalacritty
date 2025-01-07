use crate::shell_id::ShellId;

pub struct Multiplexer;

impl Multiplexer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn spawn(&mut self, _width: u32, _height: u32) -> ShellId {
        ShellId::new()
    }

    pub fn resize(&mut self, _id: ShellId, _width: u32, _height: u32) {}

    pub fn enumerate_content(&self, _id: ShellId) -> String {
        String::new()
    }

    pub fn input(&mut self, _id: ShellId, _input: &[u8]) {}
}
