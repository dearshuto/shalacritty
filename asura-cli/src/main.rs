fn main() {
    let factory = asura::DefaultFactory::default();
    let mut system = asura::TerminalSystem::new(factory);
    system.send_input("pwd\n".as_bytes()).unwrap();

    while let Ok(_) = system.receive_dirty_timeout(std::time::Duration::from_millis(100)) {
        for y in 0..24 {
            for x in 0..80 {
                let Some(cell) = system.cell(y, x) else {
                    print!(" ");
                    continue;
                };
                print!("{}", cell.ch);
            }
            println!();
        }
    }
}
