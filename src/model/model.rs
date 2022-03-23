pub trait Model {
    fn reset(&mut self);
    fn run_step(&mut self);

    fn get_elapsed_time(&self) -> f64;
    fn get_length(&self) -> &f64;
    fn get_cur_nodes(&self) -> &[f64];
    fn get_node_step(&self) -> &f64;
}
