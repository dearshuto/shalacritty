use std::time::Duration;

use renge::Service;
use tokio::task;

use crate::services::{Action, utils};

pub struct Patch {
    pub index: usize,
    pub contents: asura::Content,
}

pub struct TextData {
    pub patches: Vec<Patch>,
    pub str_count: usize,
}

pub struct ShellService {
    content_sender: tokio::sync::mpsc::Sender<TextData>,
    content_receiver: tokio::sync::mpsc::Receiver<Action>,
    old_contents: Vec<asura::Content>,
}

impl ShellService {
    pub fn new(
        content_sender: tokio::sync::mpsc::Sender<TextData>,
        content_receiver: tokio::sync::mpsc::Receiver<Action>,
    ) -> Self {
        Self {
            content_sender,
            content_receiver,
            old_contents: Vec::default(),
        }
    }

    async fn handle_contents(&mut self, contents: Vec<asura::Content>) {
        let str_count = contents.len();
        let diff_collection = utils::diff_calculator::calculate_diff(&self.old_contents, &contents);

        // TODO: パッチ化する
        let mut old_index = 0;
        let mut new_index = 0;
        let mut patches = Vec::new();
        for diff in diff_collection {
            match diff {
                utils::diff_calculator::Diff::Common(_) => {
                    old_index += 1;
                    new_index += 1;
                    continue;
                }
                utils::diff_calculator::Diff::Add(content) => {
                    patches.push(Patch {
                        index: old_index,
                        contents: content,
                    });
                    old_index += 1;
                }
                utils::diff_calculator::Diff::Remove(_) => {
                    new_index += 1;
                }
            }
        }

        self.old_contents = contents;
        self.content_sender
            .send(TextData {
                patches: Vec::default(),
                str_count,
            })
            .await
            .unwrap();
    }
}

impl Service for ShellService {
    async fn serve(mut self, mut cancellation_token: renge::CancellationToken) {
        let mut multiplexer = asura::Multiplexer::new();
        let (_id, mut shell_sender, shell_receiver) =
            multiplexer.spawn_separated(&asura::Config::default());

        let (content_sender, mut content_receiver) = tokio::sync::mpsc::channel(10);
        let task = task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(100)).await;
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
                        asura::Event::Others => continue,
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
                _ = &mut cancellation_token => break,
                else => {println!("else")}
            }
        }

        task.await.unwrap();
    }
}
