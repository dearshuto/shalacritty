use crate::types::TerminalSize;

/// 1つのセル（文字と属性）の表現
#[derive(Clone, Debug)]
pub struct Cell {
    pub ch: char,
    // 必要に応じて fg, bg, bold などのスタイル属性を追加
}

/// パーサー・VRAM（状態管理）の抽象化インターフェース
pub trait TerminalParser {
    /// PTY から流れてきた生のバイト列を処理して内部状態を更新
    fn process(&mut self, bytes: &[u8]);

    /// 画面サイズを変更
    fn set_size(&mut self, size: TerminalSize);

    /// 指定した座標 (row, col) のセル情報を取得（フロントエンド描画用）
    fn cell(&self, row: u16, col: u16) -> Option<Cell>;

    /// カーソル位置を取得 (row, col)
    fn cursor_position(&self) -> (u16, u16);
}
