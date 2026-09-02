use vt100::Cell;

use crate::services::RandomAccess;

pub struct DiffInfo {
    pub cursor: Option<(u16, u16)>,
}

pub struct Terminal {
    parser: vt100::Parser,

    cursor_position_cache: (u16, u16),

    character_cache: Vec<Cell>,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            parser: vt100::Parser::new(24, 80, 0),

            // 番兵としてあり得ない値で初期化
            cursor_position_cache: (u16::MAX, u16::MAX),

            // 初期状態は全て差分にするので空にしておく
            character_cache: Vec::new(),
        }
    }

    pub fn push(&mut self, data: &[u8]) -> DiffInfo {
        self.parser.process(data);
        let screen = self.parser.screen();

        let mut diff = DiffInfo { cursor: None };

        // カーソル位置
        let cursor = screen.cursor_position();
        if cursor.0 != self.cursor_position_cache.0 || cursor.1 != self.cursor_position_cache.1 {
            diff.cursor = Some(cursor);
            self.cursor_position_cache = cursor;
        }

        // TODO: 差分を適用するパッチを生成する
        let _diffs = crate::services::utils::calculate_diff(
            self.character_cache.as_ref(),
            SliceAdapter(screen),
        );

        diff
    }
}

struct SliceAdapter<'a>(&'a vt100::Screen);

impl RandomAccess<Cell> for SliceAdapter<'_> {
    fn len(&self) -> usize {
        let size = self.0.size();
        size.0 as usize * size.1 as usize
    }

    fn get(&self, index: usize) -> &Cell {
        let size = self.0.size();
        let x = (index % size.0 as usize) as u16;
        let y = (index / size.0 as usize) as u16;
        self.0.cell(x, y).as_ref().unwrap()
    }
}
