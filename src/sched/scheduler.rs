use web_time::Duration;

#[derive(Default)]
pub struct SchedulingInfo {
    pub num_nodes: usize,
    pub epoch_duration: Duration,
    pub priorities: Vec<i8>, /* for later use */
}

impl SchedulingInfo {
    pub fn new(num_nodes: usize) -> Self {
        Self {
            num_nodes: num_nodes, 
            epoch_duration: Duration::ZERO,
            priorities: Vec::new()
        }
    } 
}

pub trait Scheduler {
    fn get_next_node_idx(&mut self) -> usize;

    fn epoch_is_over(&self, info: &mut SchedulingInfo) -> bool;

    fn restart_epoch(&mut self, info: &mut SchedulingInfo);
}

