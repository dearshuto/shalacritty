pub enum Action<'a> {
    // 入力
    Input(&'a str),

    // 画面分割
    SplitHorizontal,

    // タブ切り替え
    ActivateTab(u32),
}
