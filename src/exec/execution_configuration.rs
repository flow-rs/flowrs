use crate::flow::flow_types::NodeId;
use std::{collections::HashMap, net::SocketAddr};
use thiserror::Error;

/// The ExecutionConfig determines for all nodes where they are executed
pub struct ExecutionConfig {
    pub node_configs: HashMap<NodeId, NodeConfig>,
}

/// The NodeConfig determines where a node is run, either local or remote
pub enum NodeConfig {
    // not sure which type of address to use
    NetworkNodeConfig(SocketAddr),
    LocalNodeConfig,
}

impl ExecutionConfig {
    pub fn new() -> Self {
        ExecutionConfig {
            node_configs: HashMap::<NodeId, NodeConfig>::new(),
        }
    }
}

#[derive(Error, Debug)]
pub enum ExecutionConfigError {
    #[error("Missing Execution Config Error. Message: {message:?}")]
    MissingExecutionConfig { message: String },
}
