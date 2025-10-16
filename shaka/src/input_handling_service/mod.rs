use winit::event::KeyEvent;

pub enum Action {
    RequestSpawn(asura::Config),
    SendInput((asura::ShellId, String)),
    ActivateTab(asura::TabId),
    ActivateShell(asura::ShellId),
}

pub struct InputHandlingService {
    input_receiver: tokio::sync::mpsc::Receiver<KeyEvent>,
    action_senders: Vec<tokio::sync::mpsc::Sender<Action>>,
    id: Option<asura::ShellId>,
}

impl InputHandlingService {
    pub fn new(input_receiver: tokio::sync::mpsc::Receiver<KeyEvent>) -> Self {
        Self {
            input_receiver,
            action_senders: Vec::default(),
            id: None,
        }
    }

    pub fn listen(&mut self) -> tokio::sync::mpsc::Receiver<Action> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        self.action_senders.push(sender);
        receiver
    }

    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        loop {
            tokio::select! {
            Some(key_event) = self.input_receiver.recv() => self.handle_input(key_event).await,
            _ = &mut cancellation_token => break,
            else => {},
            }
        }
    }

    async fn handle_input(&mut self, key_event: KeyEvent) {
        if let Some(text) = key_event.text {
            if let Some(id) = self.id {}
        }
    }

    async fn request_spawn(&mut self) {
        for sender in &self.action_senders {
            let config = asura::Config::default();
            sender.send(Action::RequestSpawn(config)).await.unwrap();
        }
    }
}

impl renge::Service for InputHandlingService {
    async fn serve(self, cancellation_token: renge::CancellationToken) {
        self.serve(cancellation_token).await;
    }
}
