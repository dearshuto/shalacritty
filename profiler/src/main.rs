use std::time::{Duration, SystemTime};

use eframe::{
    egui::{self, Color32, Pos2},
    epaint::PathStroke,
};
use profiler_core::{Client, Profile, ProfileListRequest};

#[tokio::main]
async fn main() -> eframe::Result {
    let client = Client::connect("http://localhost:3030").await.unwrap();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "shalacritty profiler",
        options,
        Box::new(|_cc| Ok(Box::new(ProfilerApp::new(client)))),
    )
}

struct ProfilerApp {
    client: Client,
    last_request_time: SystemTime,
    profile_cache: Vec<Profile>,
}

impl ProfilerApp {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            last_request_time: SystemTime::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap(),
            profile_cache: Vec::default(),
        }
    }
}

impl eframe::App for ProfilerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1 秒以上経過しないと再描画しない
        let now = SystemTime::now();
        if 1 < now
            .duration_since(self.last_request_time)
            .unwrap()
            .as_secs()
        {
            let profiles = futures::executor::block_on({
                self.client.request_profile(&ProfileListRequest {
                    id_list: Default::default(),
                })
            })
            .unwrap();
            self.profile_cache = profiles;
            self.last_request_time = now;
        }

        ctx.request_repaint_after(Duration::from_millis(1000));

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");

            let mut shapes = Vec::default();
            for profile in &self.profile_cache {
                let mut points = Vec::default();
                for (index, duration) in profile.duration().iter().enumerate() {
                    let x = (index as f32) / 1.0;
                    let y = (duration.as_nanos() as f32) / 1000.0;
                    points.push(Pos2 { x, y });
                }

                let color = Color32::from_rgb(128, 128, 128);
                let shape = egui::epaint::Shape::line(points, PathStroke::new(1.0, color));
                shapes.push(shape);
            }
            ui.painter().extend(shapes);
        });
    }
}
