pub struct ShellEvent {}

impl ShellEvent {
    pub fn is_signaled(&self) -> bool {
        return false;
    }

    pub fn wait_signaled(&mut self) {
        // TODO
        std::thread::sleep_ms(1000);
    }
}
