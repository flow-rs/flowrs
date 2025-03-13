use crate::{
    flow::flow_types::NodeId,
    sched::scheduling_config::{RuntimeId, SchedulingConfig},
};
use std::collections::HashMap;
use thiserror::Error;

/// The ExecutionConfig determines which nodes a specifc runtime is responsible for
pub struct ExecutionConfig {
    pub runtime_id: RuntimeId,
    pub node_configs: HashMap<NodeId, NodeConfig>,
}

/// The NodeConfig determines whether a node is local to this runtime, or remote
pub enum NodeConfig {
    // not sure which type of address to use
    RemoteNodeConfig(RuntimeId),
    LocalNodeConfig,
}

impl ExecutionConfig {
    pub fn new(runtime_id: RuntimeId) -> Self {
        ExecutionConfig {
            runtime_id: runtime_id,
            node_configs: HashMap::<NodeId, NodeConfig>::new(),
        }
    }

    /// Creates an ExecutionConfig for a specific runtime based on the SchedulingConfig
    pub fn from_scheduling_config(
        scheduling_config: &SchedulingConfig,
        runtime_id: RuntimeId,
    ) -> Self {
        let mut execution_config = ExecutionConfig::new(runtime_id);

        // Assign local nodes
        if let Some(local_nodes) = scheduling_config.get_nodes_for_runtime(runtime_id) {
            for &node_id in local_nodes {
                execution_config
                    .node_configs
                    .insert(node_id, NodeConfig::LocalNodeConfig);
            }
        }

        // Assign remote nodes (cross-runtime communication)
        for (other_runtime_id, nodes) in &scheduling_config.runtime_nodes {
            if *other_runtime_id != runtime_id {
                for &node_id in nodes {
                    execution_config
                        .node_configs
                        .insert(node_id, NodeConfig::RemoteNodeConfig(*other_runtime_id));
                }
            }
        }

        execution_config
    }

    pub fn is_local_connection(&self, sender_id: NodeId, receiver_id: NodeId) -> bool {
        let sender_config = self.node_configs.get(&sender_id);
        let receiver_config = self.node_configs.get(&receiver_id);
        match (sender_config, receiver_config) {
            (Some(NodeConfig::LocalNodeConfig), Some(NodeConfig::LocalNodeConfig)) => true,
            _ => false,
        }
    }
}

#[derive(Error, Debug)]
pub enum ExecutionConfigError {
    #[error("Missing Execution Config Error. Message: {message:?}")]
    MissingExecutionConfig { message: String },

    #[error("Communication Setup Failed Error. Message: {message:?}")]
    CommunicationSetupFailed { message: String },
}
