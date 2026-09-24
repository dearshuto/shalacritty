fn main() {
    let factory = asura::DefaultFactory::default();
    let mut system = asura::TerminalSystem::new(factory);

    system.send_input("ls".as_bytes()).unwrap();

    let stdin = std::io::stdin();
    let mut buffer = String::new();
    while let Ok(_) = stdin.read_line(&mut buffer) {
        if buffer.trim().is_empty() {
            break;
        }
        system.send_input(buffer.as_bytes()).unwrap();
        buffer.clear();

        while let Ok(_) = system.receive_dirty_timeout(std::time::Duration::from_millis(100)) {
            let now = std::time::SystemTime::now();
            let image = image::RgbImage::new(640, 480);
            image.save(format!("{:?}.png", now)).unwrap();

            // for y in 0..24 {
            //     for x in 0..80 {
            //         let Some(cell) = system.cell(y, x) else {
            //             print!(" ");
            //             continue;
            //         };
            //         print!("{}", cell.ch);
            //     }
            //     println!();
            // }
        }
    }
}
