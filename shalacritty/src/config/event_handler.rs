use super::Config;

pub struct EventHandler {
    sender: std::sync::mpsc::Sender<Config>,
}

impl EventHandler {
    pub fn new(sender: std::sync::mpsc::Sender<Config>) -> Self {
        Self { sender }
    }
}

impl notify::EventHandler for EventHandler {
    fn handle_event(&mut self, event: notify::Result<notify::Event>) {
        let Ok(e) = event else {
            return;
        };

        match e.kind {
            // notify::EventKind::Any => todo!(),
            // notify::EventKind::Access(_) => todo!(),
            notify::EventKind::Create(kind) => {
                if kind != notify::event::CreateKind::File {
                    return;
                }

                // 定義ファイルが作成されたので読み込む
                for path in &e.paths {
                    println!("{:?}: {:?}", e.kind, path);
                    let _file = std::fs::File::open(path).unwrap();
                    self.sender.send(Config::default()).unwrap();
                }
            }
            notify::EventKind::Modify(kind) => {
                let notify::event::ModifyKind::Data(_) = kind else {
                    return;
                };

                // 定義ファイルが更新されたので読み込む
                for path in &e.paths {
                    let config = super::util::load_config(path);
                    self.sender.send(config).unwrap();
                }
            }
            // notify::EventKind::Remove(_) => todo!(),
            // notify::EventKind::Other => todo!(),
            _ => {}
        }
    }
}
