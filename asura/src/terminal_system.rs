use std::{
    io::{Read, Write},
    sync::{Arc, Mutex},
};

use crate::{
    parser::{Cell, TerminalParser},
    pty::Pty,
};

pub trait Factory {
    type Pty: Pty;
    type Parser: TerminalParser;

    fn create_pty(&self) -> Result<Self::Pty, ()>;

    fn create_parser(&self) -> Self::Parser;
}

pub struct TerminalSystem<TParser, TWrite>
where
    TParser: TerminalParser,
    TWrite: Write,
{
    parser: Arc<Mutex<TParser>>,
    writer: TWrite,
    dirty_receiver: std::sync::mpsc::Receiver<bool>,
}

impl<TParser: TerminalParser, TWrite: Write> TerminalSystem<TParser, TWrite> {
    pub fn new<T: Factory>(factory: T) -> Self
    where
        T: Factory<Parser = TParser>,
        <T::Pty as Pty>::Reader: Send + 'static,
        T::Pty: Pty<Writer = TWrite>,
        TParser: Send + 'static,
    {
        let mut pty = factory.create_pty().unwrap();

        let mut reader = pty.reader().unwrap();
        let writer = pty.writer().unwrap();
        let parser = Arc::new(Mutex::new(factory.create_parser()));
        let parser_clone = parser.clone();

        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        parser_clone.lock().unwrap().process(&buf[..n]);
                        if let Ok(_) = sender.send(true) {
                            continue;
                        }

                        break;
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            parser,
            writer,
            dirty_receiver: receiver,
        }
    }

    pub fn send_input(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    pub fn cell(&self, row: u16, col: u16) -> Option<Cell> {
        let parser = self.parser.lock().unwrap();
        parser.cell(row, col)
    }

    pub fn receive_dirty_timeout(
        &mut self,
        timeout: std::time::Duration,
    ) -> Result<bool, std::sync::mpsc::RecvTimeoutError> {
        self.dirty_receiver.recv_timeout(timeout)
    }
}
