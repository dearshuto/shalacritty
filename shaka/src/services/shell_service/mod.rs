use renge::Service;

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<String>,
    content_receiver: tokio::sync::mpsc::Receiver<String>,
}

impl ShellService {
    pub fn new(content_sender: tokio::sync::mpsc::Sender<String>) -> Self {
        let (_content_sender, content_receiver) = tokio::sync::mpsc::channel(1);
        Self {
            content_sender,
            content_receiver,
        }
    }
}

impl Service for ShellService {
    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut multiplexer = asura::Multiplexer::new();
        let (_id, controller) = multiplexer.spawn(&asura::Config::default());
        loop {
            match controller.recv_event() {
                Ok(event) => match event {
                    asura::Event::Updated => break,
                    asura::Event::Exit => return,
                    asura::Event::Others => continue,
                },
                Err(_) => return,
            }
        }

        let content = controller
            .read_contents()
            .acquire_contents()
            .iter()
            .filter_map(|c| if c.code != ' ' { Some(c.code) } else { None })
            .take(8)
            .collect();
        self.content_sender.send(content).await.unwrap();

        loop {
            tokio::select! {
                Some(_args) = self.content_receiver.recv() => {},
                _ = &mut cancellation_token => break,
                else => {}
            }
        }
    }
}
