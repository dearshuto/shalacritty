use std::time::Duration;

use font_kit::sources::multi;
use renge::Service;
use tokio::task;

use crate::services::{
    Action, EventKind,
    utils::{self, Patch, diff_calculator::calculate_diff},
};

#[derive(Debug, Default, Copy, Clone)]
pub struct PatchData {
    pub index: usize,
    pub content: asura::Content,
}

impl Patch<asura::Content> for PatchData {
    fn new(index: usize, content: asura::Content) -> Self {
        Self { index, content }
    }
}

pub struct TextData {
    pub patches: Vec<PatchData>,
    pub char_count: usize,
}

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<TextData>,
    content_receiver: tokio::sync::mpsc::Receiver<Action>,
    event_receiver: tokio::sync::broadcast::Receiver<EventKind>,
    old_contents: Vec<asura::Content>,
}

impl ShellService {
    pub fn new(
        content_sender: tokio::sync::mpsc::Sender<TextData>,
        content_receiver: tokio::sync::mpsc::Receiver<Action>,
        event_receiver: tokio::sync::broadcast::Receiver<EventKind>,
    ) -> Self {
        Self {
            content_sender,
            content_receiver,
            event_receiver,
            old_contents: Vec::default(),
        }
    }

    async fn handle_contents(&mut self, contents: Vec<asura::Content>) {
        let diff_collection = calculate_diff(&self.old_contents, &contents);
        let patches = utils::generate_patch(diff_collection.into_iter());
        self.content_sender
            .send(TextData {
                patches,
                char_count: contents.len(),
            })
            .await
            .unwrap();

        self.old_contents = contents;
    }

    fn resize(shell_sender: &mut asura::ShellSender, event: EventKind) {
        let EventKind::Resized { width, height } = event else {
            return;
        };

        let (line_count, column_count) = utils::transform::compute_grid_size([width, height], 32.0);
        shell_sender.resize(line_count as u16, column_count as u16, 1, 1);
    }
}

impl Service for ShellService {
    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut multiplexer = asura::Multiplexer::new();
        let (id, mut shell_sender, shell_receiver) =
            multiplexer.spawn_separated(&asura::Config::default());

        let (content_sender, mut content_receiver) = tokio::sync::mpsc::channel(1);
        let task = task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(10)).await;
                match shell_receiver.try_recv_event() {
                    Ok(event) => match event {
                        asura::Event::Updated => {
                            let contents: Vec<_> = shell_receiver
                                .read_contents()
                                .acquire_contents()
                                .into_iter()
                                .filter_map(|c| if c.code != ' ' { Some(c) } else { None })
                                .collect();
                            content_sender.send(contents).await.unwrap();
                        }
                        asura::Event::Exit => continue,
                        asura::Event::Others => println!("Others"),
                    },
                    Err(_) => continue,
                }
            }
        });

        loop {
            tokio::select! {
                Some(action) = self.content_receiver.recv() => {
                    let Action::Input(str) = action else { continue };
                    shell_sender.send_input(&str);
                },
                Some(contents) = content_receiver.recv() => self.handle_contents(contents).await,
                Ok(event) = self.event_receiver.recv() => Self::resize(&mut shell_sender, event),
                _ = &mut cancellation_token => break,
                else => {println!("else")}
            }
        }

        task.await.unwrap();
    }
}
