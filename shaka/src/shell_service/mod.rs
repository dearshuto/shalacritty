pub struct SpawnRequest {
    pub config: asura::Config,
    pub ack: tokio::sync::oneshot::Sender<asura::ShellId>,
}

pub struct ShellService {
    multiplexer: asura::Multiplexer,
    spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>,
    str_diff_senders: Vec<tokio::sync::mpsc::Sender<String>>,
}

impl ShellService {
    pub fn new(spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>) -> Self {
        Self {
            multiplexer: asura::Multiplexer::new(),
            spawn_request_receiver,
            str_diff_senders: Vec::default(),
        }
    }

    pub fn listen_str_diff(&mut self) -> tokio::sync::mpsc::Receiver<String> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.str_diff_senders.push(sender);
        receiver
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                Some(request) = self.spawn_request_receiver.recv() => self.apply_spawn_request(request).await,
                _ = &mut cancellation_token => break,
                else => {}
            }
        }
    }

    async fn apply_spawn_request(&mut self, spawn_request: SpawnRequest) {
        let (id, controller) = self.multiplexer.spawn(&spawn_request.config);
    }
}

impl renge::Service for ShellService {
    async fn serve(self, mut cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
