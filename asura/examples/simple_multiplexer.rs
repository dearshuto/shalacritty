use std::{sync::mpsc::TryRecvError, time::Duration};

fn main() {
    let mut multiplexer = asura::Multiplexer::new();

    // spawn a shell
    let (_id, mut controller) = multiplexer.spawn(
        &asura::Config::default()
            .with_total_lines(10)
            .with_screen_lines(10),
    );

    controller.send_input("pwd\n");

    // 適当な間隔のポーリングでイベントが空になるまで待つ
    // 確実な方法ではないが期待した結果が得られるのでシンプルな例としてはよしとする
    loop {
        std::thread::sleep(Duration::from_millis(100));

        match controller.try_recv_event() {
            Ok(_) => continue,
            Err(error) => match error {
                TryRecvError::Disconnected => return,
                TryRecvError::Empty => break,
            },
        }
    }

    println!("pwd ==========");

    let output: String = controller
        .read_contents()
        .acquire_contents()
        .into_iter()
        .map(|c| c.code)
        .collect();
    println!("{}", output);
}
