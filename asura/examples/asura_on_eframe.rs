use std::collections::{BTreeMap, HashMap};

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Simple GUI",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

struct App {
    // ひもづいたシェルの寿命を管理するために保持しているので未使用でも意図的
    #[allow(unused)]
    multiplexer: asura::Multiplexer,

    shell_id: asura::ShellId,
    shell_controller_table: HashMap<asura::ShellId, asura::ShellController>,

    input: String,
    contents: String,
    line_coount: u16,
    column_count: u16,
    cell_width: u16,
    cell_height: u16,
}

impl App {
    pub fn new() -> Self {
        let mut multiplexer = asura::Multiplexer::new();
        let id_and_controller = multiplexer.spawn(
            &asura::Config::default()
                .with_total_lines(15)
                .with_screen_lines(15),
        );

        Self {
            multiplexer,
            shell_id: id_and_controller.0,
            shell_controller_table: HashMap::from([id_and_controller]),
            input: String::new(),
            contents: String::new(),
            line_coount: 15,
            column_count: 15,
            cell_width: 8,
            cell_height: 8,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        // 毎秒更新をかけてポーリング
        ctx.request_repaint_after_secs(1.0);

        for (_id, controller) in &self.shell_controller_table {
            let Ok(_) = controller.try_recv_event() else {
                continue;
            };

            self.contents = controller
                .read_contents()
                .acquire_contents()
                .into_iter()
                .fold(
                    BTreeMap::default(),
                    |mut tree: BTreeMap<i32, String>, value| {
                        tree.entry(value.y)
                            .or_insert_with(String::new)
                            .push(value.code);

                        tree
                    },
                )
                .into_iter()
                .map(|(_key, value)| format!("{}\n", value))
                .collect();
        }

        eframe::egui::SidePanel::right("SidePanel").show(ctx, |ui| {
            if ui
                .add(eframe::egui::Slider::new(&mut self.line_coount, 5..=60).text("line"))
                .changed()
            {
                if let Some(controller) = self.shell_controller_table.get_mut(&self.shell_id) {
                    controller.resize(
                        self.line_coount,
                        self.column_count,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }

            if ui
                .add(eframe::egui::Slider::new(&mut self.column_count, 5..=60).text("column"))
                .changed()
            {
                if let Some(controller) = self.shell_controller_table.get_mut(&self.shell_id) {
                    controller.resize(
                        self.line_coount,
                        self.column_count,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }

            if ui
                .add(eframe::egui::Slider::new(&mut self.cell_width, 2..=20).text("Cell Width"))
                .changed()
            {
                if let Some(controller) = self.shell_controller_table.get_mut(&self.shell_id) {
                    controller.resize(
                        self.line_coount,
                        self.column_count,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }

            if ui
                .add(eframe::egui::Slider::new(&mut self.cell_height, 2..=20).text("Cell Height"))
                .changed()
            {
                if let Some(controller) = self.shell_controller_table.get_mut(&self.shell_id) {
                    controller.resize(
                        self.line_coount,
                        self.column_count,
                        self.cell_width,
                        self.cell_height,
                    );
                }
            }
        });

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ui.text_edit_singleline(&mut self.input);

            if ui.button("apply").clicked() {
                self.shell_controller_table
                    .values_mut()
                    .next()
                    .unwrap()
                    .send_input(&format!("{}\n", &self.input));

                self.input = String::new();
            }

            ui.label(&self.contents);
        });
    }
}
