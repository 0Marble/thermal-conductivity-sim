use crate::model::model::*;
use exmex::prelude::*;
use rayon::prelude::*;

type T = f64;
pub struct AnalyticModel {
    func: exmex::FlatEx<T>,

    length: T,
    time_step: T,
    node_step: T,
    nodes: Vec<T>,
    cur_time_step: u32,
    node_count: u32,
}

impl AnalyticModel {
    pub fn new(func: exmex::FlatEx<T>, length: T, node_count: u32, time_step: T) -> Self {
        let node_step = length / (node_count - 1) as T;
        let nodes = (0..node_count)
            .into_par_iter()
            .map(|i| func.eval(&[0., node_step * i as T]).unwrap())
            .collect();

        Self {
            node_count,
            length,
            node_step,
            cur_time_step: 0,
            time_step,
            nodes,
            func,
        }
    }
}

impl Model for AnalyticModel {
    fn get_cur_nodes(&self) -> &[T] {
        &self.nodes[..]
    }

    fn get_length(&self) -> &T {
        &self.length
    }

    fn get_node_step(&self) -> &T {
        &self.node_step
    }

    fn reset(&mut self) {
        let func = &self.func;
        self.nodes = (0..self.node_count)
            .into_par_iter()
            .map(|i| func.eval(&[0., self.node_step * i as T]).unwrap())
            .collect();
        self.cur_time_step = 0;
    }

    fn run_step(&mut self) {
        self.cur_time_step += 1;
        self.nodes = (0..self.node_count)
            .into_par_iter()
            .map(|i| {
                self.func
                    .eval(&[
                        self.cur_time_step as T * self.time_step,
                        self.node_step * i as T,
                    ])
                    .unwrap()
            })
            .collect();
    }

    fn get_elapsed_time(&self) -> T {
        self.cur_time_step as T * self.time_step
    }
}
