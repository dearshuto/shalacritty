fn main() {
    let mut multiplexer = asura::Multiplexer::new();

    // spawn two shells
    let (id0, id1) = (multiplexer.spawn(640, 480), multiplexer.spawn(100, 100));

    multiplexer.input(id0, String::from("ls").as_bytes());
    multiplexer.input(id1, String::from("pwd").as_bytes());
}
