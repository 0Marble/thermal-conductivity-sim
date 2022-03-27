use crate::model::model::*;

use exmex::prelude::*;
use rayon::prelude::*;
extern crate lapack;
extern crate netlib_src;

pub struct SystemModel {
    starting_conditions: exmex::FlatEx<f64>,
    left_edge_conditions: exmex::FlatEx<f64>,
    right_edge_conditions: exmex::FlatEx<f64>,
    coefficient: exmex::FlatEx<f64>,
    sigma: f64,

    length: f64,
    time_step: f64,
    node_step: f64,
    nodes: Vec<f64>,
    cur_time_step: u32,
}

impl SystemModel {
    pub fn new(
        starting_conditions: exmex::FlatEx<f64>,
        left_edge_conditions: exmex::FlatEx<f64>,
        right_edge_conditions: exmex::FlatEx<f64>,
        coefficient: exmex::FlatEx<f64>,
        sigma: f64,
        length: f64,
        node_count: u32,
        time_step: f64,
    ) -> Self {
        let node_step = length / (node_count as f64 - 1.);
        let mut nodes = Vec::with_capacity(node_count as usize);
        nodes.push(left_edge_conditions.eval(&[0.]).unwrap());
        nodes.append(
            &mut (1..node_count - 1)
                .map(|i| starting_conditions.eval(&[node_step * i as f64]).unwrap())
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
            sigma,
            cur_time_step: 0,
        }
    }

    fn restore_node_value(&self, node_num: u32) -> f64 {
        if node_num == 0 {
            self.left_edge_conditions.eval(&[0.]).unwrap()
        } else if node_num == self.nodes.len() as u32 - 1 {
            self.right_edge_conditions.eval(&[0.]).unwrap()
        } else {
            self.starting_conditions
                .eval(&[self.node_step * node_num as f64])
                .unwrap()
        }
    }

    fn get_node_value(&self, node_num: u32) -> f64 {
        let time = self.cur_time_step as f64 * self.time_step;
        if node_num == 0 {
            self.left_edge_conditions.eval(&[time]).unwrap()
        } else if node_num == self.nodes.len() as u32 - 1 {
            self.right_edge_conditions.eval(&[time]).unwrap()
        } else {
            let ai = self
                .coefficient
                .eval(&[self.node_step * node_num as f64])
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

impl Model for SystemModel {
    fn get_length(&self) -> &f64 {
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

        let th = self.time_step / (self.node_step * self.node_step);
        let mut dl: Vec<f64> = (1..self.nodes.len() - 1)
            .map(|i| {
                let a = self.coefficient.eval(&[self.node_step * i as f64]).unwrap();
                -th * a * a
            })
            .collect();
        // dl.insert(0, 0.);

        let mut d: Vec<f64> = (1..self.nodes.len() - 1)
            .map(|i| {
                let a = self.coefficient.eval(&[self.node_step * i as f64]).unwrap();
                2. * th * a * a + 1.
            })
            .collect();

        let mut du: Vec<f64> = (1..self.nodes.len() - 1)
            .map(|i| {
                let a = self.coefficient.eval(&[self.node_step * i as f64]).unwrap();
                -th * a * a
            })
            .collect();

        let time = self.cur_time_step as f64 * self.time_step;
        let mut b = self.nodes.clone();
        b[0] -= self.left_edge_conditions.eval(&[time]).unwrap();
        b[self.nodes.len() - 1] -= self.right_edge_conditions.eval(&[time]).unwrap();

        unsafe {
            let mut info = 0;
            lapack::dgtsv(
                self.nodes.len() as i32 - 2,
                1,
                &mut dl,
                &mut d,
                &mut du,
                &mut b[1..self.nodes.len() - 1],
                self.nodes.len() as i32 - 2,
                &mut info,
            );

            if info != 0 {
                panic!("Info != 0");
            }
        }

        self.nodes = (0..self.nodes.len())
            .into_par_iter()
            .map(|i| self.get_node_value(i as u32))
            .zip(b.par_iter())
            .map(|(a, b)| self.sigma * b + (1. - self.sigma) * a)
            .collect();
    }

    fn get_cur_nodes(&self) -> &[f64] {
        &self.nodes[..]
    }

    fn get_node_step(&self) -> &f64 {
        &self.node_step
    }

    fn get_elapsed_time(&self) -> f64 {
        self.cur_time_step as f64 * self.time_step
    }
}
