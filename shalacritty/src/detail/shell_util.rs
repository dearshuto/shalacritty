use alacritty_terminal::event::WindowSize;

pub struct TerminalParams {
    /// フォントサイズ
    pub font_size: f32,

    /// 行間
    pub line_spacing: f32,

    /// 表示領域の横幅
    pub window_width: u32,

    /// 表示領域の縦幅
    pub window_height: u32,
}

pub fn calculate_terminal_window_size(terminal_params: &TerminalParams) -> WindowSize {
    let window_width = terminal_params.window_width as f32;
    let window_height = terminal_params.window_height as f32;

    // 縦方向はフォントサイズと行間スペースに基づいて計算
    // TODO: 本来はフォントのグリフの縦幅で最長の値を使う必要がある
    let num_lines = window_height / (terminal_params.font_size + terminal_params.line_spacing);

    // 横方向はフォント幅で決め打ちでとりあえずよしとする
    // 実際は等幅フォントでなければグリフごとに横幅が違うので、
    // 全行の中で最長の値を採用しないといけないはず
    let num_cols = window_width / terminal_params.font_size;

    // MEMO: とりあえずセル幅はフォント幅にしてそれっぽく動いているけどあってる？
    // TODO; alacritty_terminal が要求している仕様を調査する
    let cell_width = terminal_params.font_size as u16;
    let cell_height = terminal_params.font_size as u16;

    WindowSize {
        num_lines: num_lines as u16,
        num_cols: num_cols as u16,
        cell_width,
        cell_height,
    }
}
