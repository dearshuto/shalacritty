pub trait IPosition {
    #[allow(unused)]
    fn new(x: u32, y: u32) -> Self;
}

pub trait IContent {
    type TPosition: IPosition;
    #[allow(unused)]
    fn with_offset(self, offset: &Self::TPosition) -> Self;
}
