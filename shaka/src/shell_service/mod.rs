pub struct ShellService {
    str_diff_senders: Vec<tokio::sync::mpsc::Sender<String>>,
}

impl ShellService {
    pub fn new() -> Self {
        Self {
            str_diff_senders: Vec::default(),
        }
    }

    pub fn listen_str_diff(&mut self) -> tokio::sync::mpsc::Receiver<String> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.str_diff_senders.push(sender);
        receiver
    }
}
