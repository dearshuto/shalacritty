use crate::terminal::DiffInfo;

pub struct RenderingJob {
    receiver: std::sync::mpsc::Receiver<DiffInfo>,
}

impl RenderingJob {
    pub fn new(receiver: std::sync::mpsc::Receiver<()>) {}

    pub fn render(&mut self) {
        match self.receiver.try_recv() {
            Ok(_) => {}
            Err(error) => match error {
                std::sync::mpsc::TryRecvError::Empty => { /* なにもしない */ }
                std::sync::mpsc::TryRecvError::Disconnected => return,
            },
        }
    }
}
