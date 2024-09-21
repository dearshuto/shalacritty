pub enum Action<'a> {
    // 入力
    Input(&'a str),

    // ペースト
    Paste,

    // 画面分割
    SplitHorizontal,

    // タブ切り替え
    NewTab,

    // タブ切り替え
    ActivateTab(u32),

    // 入力対象の切り替え
    ActivateNextTile,

    // デバッグ情報のダンプ
    DumpDebugInfo,
}
