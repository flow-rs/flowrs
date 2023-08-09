#[derive(Default)]
pub struct SchedulingInfo {
    pub num_nodes: usize,
    pub priorities: Vec<i8>, /* for later use */
}

pub trait Scheduler {
    fn get_next_node_idx(&mut self, info: &SchedulingInfo) -> usize;

    fn epoch_is_over(&self, info: &SchedulingInfo) -> bool;

    fn restart_epoch(&mut self);
}

pub struct RoundRobinScheduler {
    cur_node_idx: usize,
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self { cur_node_idx: 0 }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn get_next_node_idx(&mut self, _info: &SchedulingInfo) -> usize {
        self.cur_node_idx += 1;

        return self.cur_node_idx - 1;
    }

    fn epoch_is_over(&self, info: &SchedulingInfo) -> bool {
        self.cur_node_idx >= info.num_nodes
    }

    fn restart_epoch(&mut self) {
        self.cur_node_idx = 0;
    }
}
