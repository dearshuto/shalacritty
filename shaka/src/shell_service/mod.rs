use std::{collections::HashMap, time::Duration};

pub struct SpawnRequest {
    pub config: asura::Config,
    pub ack: tokio::sync::oneshot::Sender<asura::ShellId>,
}

pub struct ShellService {
    multiplexer: asura::Multiplexer,
    spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>,
    str_diff_senders: Vec<tokio::sync::mpsc::Sender<String>>,
    controller_table: HashMap<asura::ShellId, asura::ShellController>,
}

impl ShellService {
    pub fn new(spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>) -> Self {
        Self {
            multiplexer: asura::Multiplexer::new(),
            spawn_request_receiver,
            str_diff_senders: Vec::default(),
            controller_table: HashMap::default(),
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
                _ = tokio::time::sleep(Duration::from_millis(500)) => self.poll_content().await,
                _ = &mut cancellation_token => break,
            }
        }
    }

    async fn apply_spawn_request(&mut self, spawn_request: SpawnRequest) {
        let (id, controller) = self.multiplexer.spawn(&spawn_request.config);
        spawn_request.ack.send(id).unwrap();
        self.controller_table.insert(id, controller);
    }

    async fn poll_content(&mut self) {
        for controller in self.controller_table.values() {
            match controller.try_recv_event() {
                Ok(event) => match event {
                    asura::Event::Updated => {
                        for content in controller.read_contents().acquire_contents() {
                            print!("{}", content.code);
                        }
                    }
                    asura::Event::Exit => todo!(),
                    asura::Event::Others => {}
                },
                Err(_) => {}
            }
        }
    }
}

impl renge::Service for ShellService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
