use crate::flow::flow_types::NodeId;
use std::collections::HashMap;

use super::scheduling_types::RuntimeId;

/// The SchedulingConfig is created by the scheduler and contains the final mapping of nodes to runtimes.
#[derive(Debug)]
pub struct SchedulingConfig {
    // Nodes assigned to each runtime
    pub runtime_nodes: HashMap<RuntimeId, Vec<NodeId>>,
}

impl SchedulingConfig {
    pub fn new() -> Self {
        SchedulingConfig {
            runtime_nodes: HashMap::new(),
        }
    }

    /// Assigns a node to a runtime
    pub fn assign_node(&mut self, runtime_id: RuntimeId, node_id: NodeId) {
        self.runtime_nodes
            .entry(runtime_id) //in place manipulation
            .or_default()
            .push(node_id);
    }

    /// Retrieves all nodes for a given runtime
    pub fn get_nodes_for_runtime(&self, runtime_id: RuntimeId) -> Option<&Vec<NodeId>> {
        self.runtime_nodes.get(&runtime_id)
    }
}
