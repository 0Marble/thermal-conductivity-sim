use crate::model::{analytic::AnalyticModel, differential::DifferentialModel, model::Model};
use exmex;

type T = f64;
pub struct ModelControls {
    start_conditions: String,
    left_edge_conditions: String,
    right_edge_conditions: String,
    coefficient: String,

    actual: String,
    compare_with_actual: bool,

    node_count: u32,
    time_step: T,
    length: T,
}

impl ModelControls {
    pub fn new() -> Self {
        Self {
            coefficient: "1+x-x".to_owned(),
            left_edge_conditions: "0+x-x".to_owned(),
            right_edge_conditions: "0+x-x".to_owned(),
            start_conditions: "100*sin(PI*x/200)".to_owned(),
            actual: "100*exp(-(PI/200)*(PI/200)*t)*sin(PI*x/200)".to_owned(),
            compare_with_actual: false,
            length: 200.,
            node_count: 100,
            time_step: 1.,
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        differential_model: &mut Option<DifferentialModel>,
        analytic_model: &mut Option<AnalyticModel>,
        min_tick_time: &mut u64,
        min_frame_time: &mut u64,
    ) {
        ui.horizontal(|ui| {
            ui.label("Starting Conditions: ");
            ui.text_edit_singleline(&mut self.start_conditions);
        });
        ui.horizontal(|ui| {
            ui.label("Left Edge: ");
            ui.text_edit_singleline(&mut self.left_edge_conditions);
        });
        ui.horizontal(|ui| {
            ui.label("Right Edge: ");
            ui.text_edit_singleline(&mut self.right_edge_conditions)
        });
        ui.horizontal(|ui| {
            ui.label("Coefficient: ");
            ui.text_edit_singleline(&mut self.coefficient);
        });
        ui.horizontal(|ui| {
            ui.label("Analytical: ");
            ui.text_edit_singleline(&mut self.actual);
        });

        ui.add(egui::Slider::new(&mut self.node_count, 3..=300).text("Node Count"));
        ui.add(egui::Slider::new(&mut self.time_step, 0.01..=10.).text("Time Step"));
        ui.add(egui::Slider::new(&mut self.length, 1.0..=400.).text("Length"));

        ui.horizontal(|ui| {
            if ui.button("Start").clicked() {
                *differential_model = Some(DifferentialModel::new(
                    exmex::parse(&self.start_conditions[..]).unwrap(),
                    exmex::parse(&self.left_edge_conditions[..]).unwrap(),
                    exmex::parse(&self.right_edge_conditions[..]).unwrap(),
                    exmex::parse(&self.coefficient[..]).unwrap(),
                    self.length,
                    self.node_count,
                    self.time_step,
                ));
                if self.compare_with_actual {
                    *analytic_model = Some(AnalyticModel::new(
                        exmex::parse(&self.actual[..]).unwrap(),
                        self.length,
                        self.node_count,
                        self.time_step,
                    ));
                }
            }

            if ui
                .checkbox(
                    &mut self.compare_with_actual,
                    "Comapre with analytical answer",
                )
                .changed()
            {
                if self.compare_with_actual {
                    *analytic_model = Some(AnalyticModel::new(
                        exmex::parse(&self.actual[..]).unwrap(),
                        self.length,
                        self.node_count,
                        self.time_step,
                    ));
                } else {
                    *analytic_model = None;
                }

                differential_model.as_mut().map(|m| m.reset());
            }
        });

        ui.add(egui::Slider::new(min_frame_time, 1..=1000).text("Target Frame Time (millisec)"));
        ui.add(egui::Slider::new(min_tick_time, 1..=100000).text("Target Tick Time (microsec)"));
    }
}

pub struct Info {}

impl Info {
    pub fn new() -> Self {
        Self {}
    }

    pub fn draw(
        &mut self,
        ui: &mut egui::Ui,
        fps: u32,
        tps: u32,
        difference: Option<T>,
        elapsed_time: Option<T>,
        is_running: &mut bool,
    ) {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(format!("FPS: {}", fps)));
            ui.add(egui::Label::new(format!("TPS: {}", tps)));
        });

        match difference {
            Some(d) => {
                ui.label(format!("Difference: {:.3}", d));
            }
            None => (),
        }

        match elapsed_time {
            Some(d) => {
                ui.label(format!("Elapsed time: {:.3}", d));
            }
            None => (),
        }

        if ui.button("Quit").clicked() {
            *is_running = false;
        }
    }
}
