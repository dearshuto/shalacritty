use std::io::{Read, Write};

use portable_pty::{CommandBuilder, MasterPty, PtySize};

use crate::pty::Pty;

pub struct PtyPortable {
    master: Box<dyn MasterPty + Send>,

    // 子プロセスのライフサイクル保持（ドロップされるとプロセスが終了するため保持しておく）
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyPortable {
    pub fn new(rows: u16, cols: u16) -> Result<Self, ()> {
        // 1. ネイティブ OS の PTY システムを取得 (Linux/macOS/Windows ConPTY を自動選択)
        let pty_system = portable_pty::native_pty_system();

        // 2. 指定されたサイズで PTY ペア (master / slave) を作成
        let Ok(pair) = pty_system.openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        }) else {
            return Err(());
        };

        // 3. 起動するコマンド（bash や zsh、powershell 等）を設定
        let cmd = CommandBuilder::new("bash");
        // ターミナル種別を環境変数として設定しておくとシェルが正しく動作します
        // cmd.env("TERM", "xterm-256color");

        // 4. slave 側で子プロセスを起動
        let Ok(child) = pair.slave.spawn_command(cmd) else {
            return Err(());
        };

        Ok(Self {
            master: pair.master,
            _child: child,
        })
    }
}

impl Pty for PtyPortable {
    type Writer = Box<dyn Write + Send>;
    type Reader = Box<dyn Read + Send>;

    fn writer(&mut self) -> Result<Self::Writer, ()> {
        let Ok(writer) = self.master.take_writer() else {
            return Err(());
        };

        Ok(writer)
    }

    fn reader(&mut self) -> Result<Self::Reader, ()> {
        let Ok(reader) = self.master.try_clone_reader() else {
            return Err(());
        };

        Ok(reader)
    }

    fn resize(&mut self, size: crate::types::TerminalSize) -> Result<(), ()> {
        let Ok(result) = self.master.resize(PtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: 0,
            pixel_height: 0,
        }) else {
            return Err(());
        };

        Ok(result)
    }
}
