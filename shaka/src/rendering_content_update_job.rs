use crate::terminal::DiffInfo;

struct BufferAccessor;

struct CharacterData;

impl BufferAccessor {
    pub fn write(&mut self, _index: usize, _data: &CharacterData) {}
}

pub struct Job {
    receiver: std::sync::mpsc::Receiver<DiffInfo>,

    buffer_accessor_receiver: std::sync::mpsc::Receiver<BufferAccessor>,
}

impl Job {
    pub fn serve(self) {
        while let Ok(_) = self.receiver.recv() {
            let Ok(mut buffer_accessor) = self.buffer_accessor_receiver.recv() else {
                break;
            };

            buffer_accessor.write(0, &CharacterData {});
        }
    }
}
