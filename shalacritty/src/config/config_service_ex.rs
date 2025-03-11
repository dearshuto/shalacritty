use std::{sync::Arc, time::Duration};

use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        RwLock,
    },
    task::JoinHandle,
};

use super::{detail::EventHandler, Config};

pub struct ConfigServiceEx {
    // drop のタイミングを制御したいので Option でくるんでいる
    event_handler: EventHandler,

    senders: Arc<RwLock<Vec<tokio::sync::mpsc::Sender<Config>>>>,

    watch_task_handle: JoinHandle<()>,
}

impl ConfigServiceEx {
    pub fn new(runtime: Arc<tokio::runtime::Runtime>) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let config_dir = super::util::create_config_directory();
        let event_handler = EventHandler::new(&config_dir, sender);

        let senders = Arc::new(RwLock::new(
            Vec::<tokio::sync::mpsc::Sender<Config>>::default(),
        ));
        let senders_local = senders.clone();
        let watch_task_handle = runtime.spawn(async move {
            loop {
                println!("a");
                let Ok(config) = receiver.recv() else {
                    println!("b");
                    break;
                };

                println!("notify: Ex");
                for sender in senders_local.read().await.iter() {
                    sender.send(config.clone()).await.unwrap();
                }
            }
        });

        Self {
            event_handler,
            senders,
            watch_task_handle,
        }
    }

    pub async fn listen(&mut self) -> Receiver<Config> {
        let (sender, receiver) = tokio::sync::mpsc::channel(1);

        let mut senders = self.senders.write().await;
        senders.push(sender);

        receiver
    }
}

impl Drop for ConfigServiceEx {
    fn drop(&mut self) {
        while self.watch_task_handle.is_finished() {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::ConfigServiceEx;

    #[test]
    fn new() {
        let runtime = Arc::new(tokio::runtime::Builder::new_multi_thread().build().unwrap());

        let mut config_service = ConfigServiceEx::new(runtime.clone());
        let _receiver = runtime.block_on(async { config_service.listen().await });
    }
}
