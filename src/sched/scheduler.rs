use std::{time::{Duration, Instant}};

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

pub struct RoundRobinScheduler {
    cur_node_idx: usize,
    last_restart: Instant
}

impl RoundRobinScheduler {
    pub fn new() -> Self {
        Self { 
            cur_node_idx: 0,   
            last_restart: Instant::now()
        }
    }
}

impl Scheduler for RoundRobinScheduler {
    fn get_next_node_idx(&mut self) -> usize {
        self.cur_node_idx += 1;

        return self.cur_node_idx - 1;
    }

    fn epoch_is_over(&self, info: &mut SchedulingInfo) -> bool {
        if self.cur_node_idx >= info.num_nodes {
            info.epoch_duration = self.last_restart.elapsed();
            true
        }
        else {
            false            
        }
    }

    fn restart_epoch(&mut self, _info: &mut SchedulingInfo) {
        self.cur_node_idx = 0;
        self.last_restart = Instant::now();
    }
}
