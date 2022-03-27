use crate::model::model::*;

use exmex::prelude::*;
use rayon::prelude::*;

type T = f64;
pub struct DifferentialModel {
    starting_conditions: exmex::FlatEx<T>,
    left_edge_conditions: exmex::FlatEx<T>,
    right_edge_conditions: exmex::FlatEx<T>,
    coefficient: exmex::FlatEx<T>,

    length: T,
    time_step: T,
    node_step: T,
    nodes: Vec<T>,
    cur_time_step: u32,
}

impl DifferentialModel {
    pub fn new(
        starting_conditions: exmex::FlatEx<T>,
        left_edge_conditions: exmex::FlatEx<T>,
        right_edge_conditions: exmex::FlatEx<T>,
        coefficient: exmex::FlatEx<T>,
        length: T,
        node_count: u32,
        time_step: T,
    ) -> Self {
        let node_step = length / (node_count as T - 1.);
        let mut nodes = Vec::with_capacity(node_count as usize);
        nodes.push(left_edge_conditions.eval(&[0.]).unwrap());
        nodes.append(
            &mut (1..node_count - 1)
                .map(|i| starting_conditions.eval(&[node_step * i as T]).unwrap())
                .collect(),
        );
        nodes.push(right_edge_conditions.eval(&[0.]).unwrap());
        Self {
            node_step,
            coefficient,
            left_edge_conditions,
            right_edge_conditions,
            starting_conditions,
            length,
            time_step,
            nodes,
            cur_time_step: 0,
        }
    }

    fn restore_node_value(&self, node_num: u32) -> T {
        if node_num == 0 {
            self.left_edge_conditions.eval(&[0.]).unwrap()
        } else if node_num == self.nodes.len() as u32 - 1 {
            self.right_edge_conditions.eval(&[0.]).unwrap()
        } else {
            self.starting_conditions
                .eval(&[self.node_step * node_num as T])
                .unwrap()
        }
    }

    fn get_node_value(&self, node_num: u32) -> T {
        let time = self.cur_time_step as T * self.time_step;
        if node_num == 0 {
            self.left_edge_conditions.eval(&[time]).unwrap()
        } else if node_num == self.nodes.len() as u32 - 1 {
            self.right_edge_conditions.eval(&[time]).unwrap()
        } else {
            let ai = self
                .coefficient
                .eval(&[self.node_step * node_num as T])
                .unwrap();

            let a2 = ai * ai;
            let h2 = self.node_step * self.node_step;

            let res = a2 * self.time_step / h2
                * (self.nodes[(node_num - 1) as usize] - 2. * self.nodes[node_num as usize]
                    + self.nodes[(node_num + 1) as usize])
                + self.nodes[node_num as usize];
            res
        }
    }
}

impl Model for DifferentialModel {
    fn get_length(&self) -> &T {
        &self.length
    }

    fn reset(&mut self) {
        let nodes = (0..self.nodes.len())
            .into_par_iter()
            .map(|i| self.restore_node_value(i as u32))
            .collect();

        self.cur_time_step = 0;

        self.nodes = nodes;
    }

    fn run_step(&mut self) {
        self.cur_time_step += 1;

        self.nodes = (0..self.nodes.len())
            .into_par_iter()
            .map(|i| self.get_node_value(i as u32))
            .collect();
    }

    fn get_cur_nodes(&self) -> &[T] {
        &self.nodes[..]
    }

    fn get_node_step(&self) -> &T {
        &self.node_step
    }

    fn get_elapsed_time(&self) -> T {
        self.cur_time_step as T * self.time_step
    }
}
