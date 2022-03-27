use std::{collections::HashMap, rc::Rc, time::Duration};

use crate::model::{
    analytic::AnalyticModel, differential::DifferentialModel, model::Model, system::SystemModel,
};
use egui;
use exmex::prelude::*;

use super::model_manager::ModelInfo;

pub trait Reducer<POST, GET> {
    fn reduce(&mut self, op: POST);
    fn request(&mut self, op: &mut GET);
}

pub enum UiPost {
    AddModel(String, Box<dyn Model>),
    RemoveModel(String),
    StartComparison(String, String),
    StopComparison(String, String),
    RestartModel(String),
    SetMinTickTime(Duration),
    SetMinFrameTime(Duration),
}

pub enum UiGet {
    ModelInfo(Option<Rc<Vec<ModelInfo>>>),
    GetTps(Option<usize>),
    GetFps(Option<usize>),
}

fn make_expr(
    expr_str: &str,
    error_message: &str,
    expected_args: usize,
    error_accumulator: &mut Option<String>,
) -> exmex::FlatEx<f64> {
    let mut expr = exmex::parse::<f64>(expr_str).unwrap_or_else(|e| {
        *error_accumulator = Some(format!(
            "{}{}: {}\n",
            error_accumulator.as_ref().unwrap_or(&"".to_owned()),
            error_message,
            e
        ));
        make_expr("x-x", error_message, expected_args, error_accumulator)
    });
    if expr.var_names().len() < expected_args {
        let new_expr = (0..(expected_args - expr.var_names().len()))
            .map(|n| format!("+arg{}-arg{}", n, n))
            .fold(expr_str.to_owned(), |acc, elem| acc + &elem);
        expr = exmex::parse::<f64>(&new_expr).unwrap();
    } else if expr.var_names().len() > expected_args {
        *error_accumulator = Some(format!(
            "{}{}: too many arguments, expected{}\n",
            error_accumulator.as_ref().unwrap_or(&"".to_owned()),
            error_message,
            expected_args
        ));
    }
    expr
}
pub struct Controls {
    start_conditions: String,
    left_edge_conditions: String,
    right_edge_conditions: String,
    coefficient: String,
    actual: String,
    node_count: u32,
    time_step: f64,
    length: f64,
    sigma: f64,
    model_name: String,
    add_comparison: HashMap<String, String>,
    min_tick_time: u64,
    min_frame_time: u64,

    errors: Option<String>,
}

impl Controls {
    pub fn new() -> Self {
        Self {
            coefficient: "1".to_owned(),
            left_edge_conditions: "0".to_owned(),
            right_edge_conditions: "0".to_owned(),
            start_conditions: "100*sin(PI*x/200)".to_owned(),
            actual: "100*exp(-(PI/200)*(PI/200)*t)*sin(PI*x/200)".to_owned(),
            length: 200.,
            node_count: 100,
            time_step: 1.,
            sigma: 0.5,
            model_name: String::new(),
            add_comparison: HashMap::new(),
            errors: None,
            min_frame_time: 10,
            min_tick_time: 1,
        }
    }

    pub fn draw(&mut self, ctx: &egui::CtxRef, reducer: &mut dyn Reducer<UiPost, UiGet>) {
        egui::Window::new("Model Creator").show(ctx, |ui| self.draw_model_creator(ui, reducer));
        egui::Window::new("Current Models").show(ctx, |ui| self.draw_model_list(ui, reducer));
        egui::Window::new("Info").show(ctx, |ui| self.draw_info(ui, reducer));
    }

    fn draw_model_creator(&mut self, ui: &mut egui::Ui, reducer: &mut dyn Reducer<UiPost, UiGet>) {
        ui.horizontal(|ui| {
            ui.label("Model name: ");
            ui.text_edit_singleline(&mut self.model_name);
        });
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
        ui.add(egui::Slider::new(&mut self.sigma, 0.0..=1.0).text("Sigma"));

        if ui.button("Add Differential Model").clicked() {
            self.errors = None;
            let sc = make_expr(
                &self.start_conditions[..],
                "Invalid start conditions field",
                1,
                &mut self.errors,
            );
            let lc = make_expr(
                &self.left_edge_conditions[..],
                "Invalid left edge conditions",
                1,
                &mut self.errors,
            );
            let rc = make_expr(
                &self.right_edge_conditions[..],
                "Invalid right edge coditions",
                1,
                &mut self.errors,
            );
            let c = make_expr(
                &self.coefficient[..],
                "Invalid coefficient field",
                1,
                &mut self.errors,
            );

            if self.model_name.len() == 0 {
                self.errors = Some(format!(
                    "{}Invalid model name field: no model name\n",
                    &self.errors.as_ref().unwrap_or(&"".to_owned())
                ));
            }

            if self.errors.is_none() {
                let model = Box::new(DifferentialModel::new(
                    sc,
                    lc,
                    rc,
                    c,
                    self.length,
                    self.node_count,
                    self.time_step,
                ));
                reducer.reduce(UiPost::AddModel(self.model_name.clone(), model));
                self.add_comparison
                    .insert(self.model_name.clone(), "".to_owned());
                self.model_name.clear();
                self.errors = None;
            }
        }

        if ui.button("Add Analytic").clicked() {
            self.errors = None;

            let f = make_expr(
                &self.actual[..],
                "Invalid actual field",
                2,
                &mut self.errors,
            );
            if self.model_name.len() == 0 {
                self.errors = Some(format!(
                    "{}Invalid model name field: no model name\n",
                    &self.errors.as_ref().unwrap_or(&"".to_owned())
                ));
            }

            if self.errors.is_none() {
                let m = Box::new(AnalyticModel::new(
                    f,
                    self.length,
                    self.node_count,
                    self.time_step,
                ));
                reducer.reduce(UiPost::AddModel(self.model_name.clone(), m));
                self.add_comparison
                    .insert(self.model_name.clone(), "".to_owned());
                self.model_name.clear();
                self.errors = None;
            }
        }

        if ui.button("Add System").clicked() {
            self.errors = None;
            let sc = make_expr(
                &self.start_conditions[..],
                "Invalid start conditions field",
                1,
                &mut self.errors,
            );
            let lc = make_expr(
                &self.left_edge_conditions[..],
                "Invalid left edge conditions",
                1,
                &mut self.errors,
            );
            let rc = make_expr(
                &self.right_edge_conditions[..],
                "Invalid right edge coditions",
                1,
                &mut self.errors,
            );
            let c = make_expr(
                &self.coefficient[..],
                "Invalid coefficient field",
                1,
                &mut self.errors,
            );

            if self.model_name.len() == 0 {
                self.errors = Some(format!(
                    "{}Invalid model name field: no model name\n",
                    &self.errors.as_ref().unwrap_or(&"".to_owned())
                ));
            }

            if self.errors.is_none() {
                let model = Box::new(SystemModel::new(
                    sc,
                    lc,
                    rc,
                    c,
                    self.sigma,
                    self.length,
                    self.node_count,
                    self.time_step,
                ));
                reducer.reduce(UiPost::AddModel(self.model_name.clone(), model));
                self.add_comparison
                    .insert(self.model_name.clone(), "".to_owned());
                self.model_name.clear();
                self.errors = None;
            }
        }

        if let Some(e) = &self.errors {
            ui.label(e);
        }
    }

    fn draw_model_list(&mut self, ui: &mut egui::Ui, reducer: &mut dyn Reducer<UiPost, UiGet>) {
        let mut removed_models = vec![];
        let mut removed_comparisons = vec![];

        let mut m = UiGet::ModelInfo(None);
        reducer.request(&mut m);
        let model_info = match m {
            UiGet::ModelInfo(m) => m.unwrap(),
            _ => panic!("Expected a vec of model info"),
        };

        for model in model_info.iter() {
            let name = &model.name;

            ui.horizontal(|ui| {
                ui.label(name);
                if ui.button("↺").clicked() {
                    reducer.reduce(UiPost::RestartModel(name.clone()));
                }
                if ui.button("🗑").clicked() {
                    removed_models.push(name.clone());
                }
            });
            ui.horizontal(|ui| {
                let n2 = self.add_comparison.get_mut(name).unwrap();
                ui.text_edit_singleline(n2);
                if ui.button("Start Comparing").clicked() {
                    reducer.reduce(UiPost::StartComparison(name.clone(), n2.clone()));
                    *n2 = "".to_owned();
                }
            });

            for (comp_name, difference) in &model.comparisons {
                ui.horizontal(|ui| {
                    ui.label(format!("Difference with {}: {:.4}", comp_name, difference));
                    if ui.button("↺").clicked() {
                        reducer.reduce(UiPost::StartComparison(name.clone(), comp_name.clone()));
                    }
                    if ui.button("🗑").clicked() {
                        removed_comparisons.push((name.clone(), comp_name.clone()));
                    }
                });
            }
            ui.separator();
        }

        for model_name in &removed_models {
            reducer.reduce(UiPost::RemoveModel(model_name.clone()));
        }

        for (n1, n2) in &removed_comparisons {
            reducer.reduce(UiPost::StopComparison(n1.clone(), n2.clone()));
        }
    }

    pub fn draw_info(&mut self, ui: &mut egui::Ui, reducer: &mut dyn Reducer<UiPost, UiGet>) {
        if ui
            .add(
                egui::Slider::new(&mut self.min_tick_time, 1..=10000)
                    .text("Min Tick Time (microsec)"),
            )
            .changed()
        {
            reducer.reduce(UiPost::SetMinTickTime(Duration::from_micros(
                self.min_tick_time,
            )));
        }

        let mut tps = UiGet::GetTps(None);
        reducer.request(&mut tps);
        let tps = match tps {
            UiGet::GetTps(tps) => tps.unwrap(),
            _ => panic!("Expeced GetTps"),
        };

        ui.label(format!("TPS: {}", tps));
    }
}
