pub enum Action<'a> {
    Input(&'a str),
    SplitHorizontal,
}
