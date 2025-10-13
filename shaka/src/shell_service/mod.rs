use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

pub struct SpawnRequest {
    pub config: asura::Config,
    pub ack: tokio::sync::oneshot::Sender<asura::ShellId>,
}

pub struct ShellService {
    terminal_emulator: asura::TerminalEmulator,
    tab_id: Vec<asura::TabId>,
    shell_ids: HashMap<asura::ShellId, asura::DiffContext>,

    spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>,
    diff_sender: tokio::sync::mpsc::Sender<asura::Diff>,
    str_diff_senders: Vec<tokio::sync::mpsc::Sender<HashSet<char>>>,
    controller_table: HashMap<asura::ShellId, asura::ShellController>,
}

impl ShellService {
    pub fn new(
        spawn_request_receiver: tokio::sync::mpsc::Receiver<SpawnRequest>,
    ) -> (Self, tokio::sync::mpsc::Receiver<asura::Diff>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        let (tab_id, shell_id, terminal_emulator) = asura::TerminalEmulator::new();

        (
            Self {
                terminal_emulator,
                tab_id: vec![tab_id],
                shell_ids: HashMap::from([(shell_id, asura::DiffContext::new())]),
                spawn_request_receiver,
                diff_sender: sender,
                str_diff_senders: Vec::default(),
                controller_table: HashMap::default(),
            },
            receiver,
        )
    }

    pub fn listen_str_diff(&mut self) -> tokio::sync::mpsc::Receiver<HashSet<char>> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.str_diff_senders.push(sender);
        receiver
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
                Some(request) = self.spawn_request_receiver.recv() => self.apply_spawn_request(request).await,
                _ = tokio::time::sleep(Duration::from_millis(20)) => self.poll_content().await,
                _ = &mut cancellation_token => break,
            }
        }
    }

    async fn apply_spawn_request(&mut self, _spawn_request: SpawnRequest) {
        //
    }

    async fn poll_content(&mut self) {
        // まずは更新をかける
        self.terminal_emulator.update();

        let mut str: HashSet<char, _> = HashSet::new();
        for (shell_id, context) in &mut self.shell_ids {
            let Some(is_dirty) = self.terminal_emulator.is_dirty(*shell_id) else {
                continue;
            };

            if !is_dirty {
                continue;
            }

            let diff = self.terminal_emulator.diff(*shell_id, context);
            for diff in &diff.content_diff {
                match diff {
                    asura::DiffType::Add(diff_content) => {
                        str.insert(diff_content.content.code);
                    }
                    asura::DiffType::Update(_diff_content) => {}
                    asura::DiffType::Remove(_) => {}
                }
            }
        }

        if str.is_empty() {
            return;
        }

        for sender in &self.str_diff_senders {
            sender.send(str.clone()).await.unwrap();
        }
    }
}

impl renge::Service for ShellService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
