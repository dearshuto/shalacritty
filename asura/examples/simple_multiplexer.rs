fn main() {
    let mut multiplexer = asura::Multiplexer::new();

    // spawn a shell
    let id = multiplexer.spawn(640, 480);
    let mut shell_event = multiplexer.subscribe_shell_event(id);

    multiplexer.input(id, String::from("pwd\n").as_bytes());
    shell_event.wait_signaled();

    println!("pwd ==========");
    println!("{}", multiplexer.enumerate_content(id).unwrap());
}
