use std::io::{Read, Write};

use crate::types::TerminalSize;

/// PTY（プロセス通信）の抽象化インターフェース
pub trait Pty: Send {
    type Writer: Write + Send;
    type Reader: Read + Send;

    /// 書き込みストリームを取得します。
    fn writer(&mut self) -> Result<Self::Writer, ()>;

    /// 読み込みストリームを取得します。
    fn reader(&mut self) -> Result<Self::Reader, ()>;

    /// リサイズ通知
    fn resize(&mut self, size: TerminalSize) -> Result<(), ()>;
}
