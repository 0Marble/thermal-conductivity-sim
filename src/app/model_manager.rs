use crate::model::model::Model;
use crate::ticker::Ticker;
use petgraph::{prelude::*, visit::IntoNodeReferences};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{spawn, JoinHandle},
    time::Duration,
};

fn compare_models(model_1: &Box<dyn Model>, model_2: &Box<dyn Model>) -> f64 {
    model_1
        .get_cur_nodes()
        .par_iter()
        .zip(model_2.get_cur_nodes().par_iter())
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

enum MessageToThread {
    SetMinTickTime(Duration),
    AddModel(String, Box<dyn Model>),
    RemoveModel(String),
    StartComparison(String, String),
    StopComparison(String, String),
    Exit,
    RequestNodes,
    RestartModel(String),
}

pub struct ModelInfo {
    pub name: String,
    pub nodes: Vec<f64>,
    pub length: f64,
    pub comparisons: HashMap<String, f64>,
}

enum MessageFromThread {
    SendInfo((Vec<ModelInfo>, usize)),
}

pub struct ModelManager {
    physics_thread: Option<JoinHandle<()>>,
    tx: Sender<MessageToThread>,
    rx: Receiver<MessageFromThread>,
}

impl ModelManager {
    pub fn new(min_tick_time: Duration) -> Self {
        let (tx_from_thread, rx_from_thread) = channel();
        let (tx_from_main, rx_from_main) = channel();

        let physics_thread = spawn(move || {
            let mut models = HashMap::new();
            let tx = tx_from_thread;
            let rx = rx_from_main;
            let mut is_running = true;
            let mut comparisons = UnGraph::<String, f64>::new_undirected();
            let mut ticker = Ticker::new(min_tick_time);

            while is_running {
                ticker.start_tick();

                let mut send_info = false;
                match rx.try_recv() {
                    Err(e) => match e {
                        std::sync::mpsc::TryRecvError::Disconnected => {
                            panic!("Other side disconnected")
                        }
                        std::sync::mpsc::TryRecvError::Empty => (),
                    },
                    Ok(m) => match m {
                        MessageToThread::StartComparison(n1, n2) => {
                            let (a, _) = comparisons
                                .node_references()
                                .filter(|(_, n)| &n[..] == &n1[..])
                                .last()
                                .unwrap();
                            let (b, _) = comparisons
                                .node_references()
                                .filter(|(_, n)| &n[..] == &n2[..])
                                .last()
                                .unwrap();
                            comparisons.update_edge(a, b, 0.0);
                            models.get_mut(&n1).map(|m: &mut Box<dyn Model>| m.reset());
                            models.get_mut(&n2).map(|m| m.reset());
                        }
                        MessageToThread::StopComparison(n1, n2) => {
                            let (a, _) = comparisons
                                .node_references()
                                .filter(|(_, n)| &n[..] == &n1[..])
                                .last()
                                .unwrap();
                            let b = comparisons
                                .neighbors(a)
                                .filter(|b| comparisons.node_weight(*b).unwrap() == &n2)
                                .last()
                                .unwrap();
                            comparisons.remove_edge(
                                comparisons.edges_connecting(a, b).last().unwrap().id(),
                            );
                        }
                        MessageToThread::Exit => {
                            is_running = false;
                        }
                        MessageToThread::RestartModel(s) => {
                            models.get_mut(&s).map(|m| m.reset());
                        }
                        MessageToThread::AddModel(s, m) => {
                            if comparisons
                                .node_references()
                                .find(|(_, n)| &n[..] == &s[..])
                                .is_none()
                            {
                                models.insert(s.clone(), m);
                                comparisons.add_node(s);
                            }
                        }
                        MessageToThread::RemoveModel(s) => {
                            let n = comparisons
                                .node_references()
                                .filter(|(_, n)| &n[..] == &s[..])
                                .last();
                            match n {
                                Some((a, _)) => {
                                    comparisons.remove_node(a);
                                    models.remove(&s);
                                }
                                None => (),
                            }
                        }
                        MessageToThread::RequestNodes => send_info = true,
                        MessageToThread::SetMinTickTime(t) => ticker.set_min_tick_time(t),
                    },
                }

                models.iter_mut().for_each(|(_, m)| m.run_step());
                comparisons.edge_indices().for_each(|e| {
                    let (n1, n2) = comparisons.edge_endpoints(e).unwrap();
                    let m1 = comparisons.node_weight(n1).unwrap();
                    let m2 = comparisons.node_weight(n2).unwrap();
                    let new_diff =
                        compare_models(&models.get(m1).unwrap(), &models.get(m2).unwrap());
                    *comparisons.edge_weight_mut(e).unwrap() = new_diff;
                });

                if send_info {
                    let info = (comparisons.node_references().map(|(a, n1)| ModelInfo {
                        name: n1.clone(),
                        length: models.get(n1).unwrap().get_length().clone(),
                        nodes: Vec::from(models.get(n1).unwrap().get_cur_nodes().clone()),
                        comparisons: comparisons
                            .edges(a)
                            .map(|e| {
                                (
                                    comparisons.node_weight(e.target()).unwrap().clone(),
                                    e.weight().clone(),
                                )
                            })
                            .collect(),
                    }))
                    .collect();

                    tx.send(MessageFromThread::SendInfo((info, ticker.get_tps())))
                        .unwrap();
                }

                ticker.end_tick();
            }
        });
        Self {
            physics_thread: Some(physics_thread),
            tx: tx_from_main,
            rx: rx_from_thread,
        }
    }
    pub fn add_model(&self, name: &str, model: Box<dyn Model>) {
        self.tx
            .send(MessageToThread::AddModel(name.to_owned(), model))
            .unwrap();
    }
    pub fn remove_model(&self, name: &str) {
        self.tx
            .send(MessageToThread::RemoveModel(name.to_owned()))
            .unwrap();
    }

    pub fn get_info(&self) -> (Vec<ModelInfo>, usize) {
        self.tx.send(MessageToThread::RequestNodes).unwrap();
        match self.rx.recv().unwrap() {
            MessageFromThread::SendInfo(n) => n,
        }
    }
    pub fn set_min_tick_time(&self, min_tick_time: Duration) {
        self.tx
            .send(MessageToThread::SetMinTickTime(min_tick_time))
            .unwrap();
    }
    pub fn start_comparison(&self, model_1: &str, model_2: &str) {
        self.tx
            .send(MessageToThread::StartComparison(
                model_1.to_owned(),
                model_2.to_owned(),
            ))
            .unwrap();
    }
    pub fn stop_comparison(&self, model_1: &str, model_2: &str) {
        self.tx
            .send(MessageToThread::StopComparison(
                model_1.to_owned(),
                model_2.to_owned(),
            ))
            .unwrap();
    }
    pub fn restart_model(&self, model: &str) {
        self.tx
            .send(MessageToThread::RestartModel(model.to_owned()))
            .unwrap();
    }
}

impl Drop for ModelManager {
    fn drop(&mut self) {
        self.tx.send(MessageToThread::Exit).unwrap();
        self.physics_thread.take().map(|t| t.join());
    }
}
