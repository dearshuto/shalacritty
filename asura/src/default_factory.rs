use crate::detail::{ParserVt100, PtyPortable};
use crate::terminal_system::Factory;

#[derive(Default)]
pub struct DefaultFactory;

impl Factory for DefaultFactory {
    type Pty = PtyPortable;
    type Parser = ParserVt100;

    fn create_pty(&self) -> Result<Self::Pty, ()> {
        PtyPortable::new(80, 24)
    }

    fn create_parser(&self) -> Self::Parser {
        ParserVt100::new()
    }
}
