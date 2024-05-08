use bitflags::bitflags;

bitflags! {
    pub struct Modifier: u8 {
        const CONTROL = 0b00000001;
    }
}

pub struct Input<'a> {
    pub modifiers: Modifier,
    pub text: &'a str,
}
