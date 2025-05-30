use crate::flow::flow::Flow;

use super::{
    infrastructure_config::InfrastructureConfig, scheduler::Scheduler,
    scheduling_config::SchedulingConfig,
};

pub struct RoundRobinScheduler {
    infra: InfrastructureConfig,
}

impl Scheduler for RoundRobinScheduler {
    fn new(infra: InfrastructureConfig) -> Self {
        RoundRobinScheduler { infra }
    }

    fn schedule(&self, flow: &Flow) -> SchedulingConfig {
        let mut config = SchedulingConfig::new();

        // Get all runtime IDs excluding the orchestrator (id 0)
        let mut runtime_ids: Vec<_> = self
            .infra
            .machines
            .iter()
            .filter(|m| m.runtime_id != 0)
            .map(|m| m.runtime_id)
            .collect();

        runtime_ids.sort(); // Ensure stable ordering

        let mut index = 0;
        let node_ids: Vec<_> = flow.get_nodes().map(|(id, _)| *id).collect();

        for node_id in node_ids {
            let runtime_id = runtime_ids[index % runtime_ids.len()];
            config.assign_node(runtime_id, node_id);
            index += 1;
        }

        config
    }
}
