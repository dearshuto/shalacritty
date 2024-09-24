use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use eframe::{
    egui::{self, Color32, Pos2},
    epaint::PathStroke,
};
use profiler_core::{Client, Profile, ProfileListRequest};

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "shalacritty profiler",
        options,
        Box::new(|_cc| Ok(Box::new(ProfilerApp::new()))),
    )
    .unwrap();
}

struct ProfilerApp {
    runtime: tokio::runtime::Runtime,
    client: Arc<Client>,
    last_request_time: SystemTime,
    profile_cache: Arc<Mutex<Vec<Profile>>>,
    visibility_table: HashMap<String, bool>,
    scale_y: f32,
}

impl ProfilerApp {
    pub fn new() -> Self {
        let client = Client::connect("http://localhost:3030").unwrap();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        Self {
            runtime,
            client: Arc::new(client),
            last_request_time: SystemTime::now()
                .checked_sub(Duration::from_secs(10))
                .unwrap(),
            profile_cache: Default::default(),
            visibility_table: HashMap::default(),
            scale_y: 0.001,
        }
    }
}

impl eframe::App for ProfilerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(30));

        // 1 秒以上経過しないと再描画しない
        let now = SystemTime::now();
        if 30
            < now
                .duration_since(self.last_request_time)
                .unwrap()
                .as_millis()
        {
            let client = self.client.clone();
            let profile_cache = self.profile_cache.clone();
            let _handle = self.runtime.spawn(async move {
                let new_profile = client
                    .request_profile(&ProfileListRequest {
                        id_list: Default::default(),
                    })
                    .await
                    .unwrap();
                *profile_cache.lock().unwrap() = new_profile;
            });

            self.last_request_time = now;
        }

        let profile_cache = self.profile_cache.lock().unwrap();
        egui::SidePanel::right("_").show(ctx, |ui| {
            ui.add(egui::Slider::new(&mut self.scale_y, 0.0001..=1.0).text("Scale Y"));

            for profile in profile_cache.iter() {
                let mut is_visible = if self.visibility_table.contains_key(profile.name()) {
                    *self.visibility_table.get(profile.name()).unwrap()
                } else {
                    self.visibility_table
                        .insert(profile.name().to_string(), true);
                    true
                };
                if ui.checkbox(&mut is_visible, profile.name()).changed() {
                    self.visibility_table
                        .insert(profile.name().to_string(), is_visible);
                }
            }
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");

            let mut shapes = Vec::default();
            for profile in profile_cache.iter() {
                // 不可視のグラフを消す
                if self.visibility_table.contains_key(profile.name())
                    && !*self.visibility_table.get(profile.name()).unwrap()
                {
                    continue;
                }

                let mut points = Vec::default();
                for (index, duration) in profile.duration().iter().enumerate() {
                    let x = (index as f32) / 1.0;

                    // nsec -> msec
                    let y = 100.0 + (duration.as_nanos() as f32 / 1000.0) * self.scale_y;
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
