use std::collections::{hash_set::Iter, HashSet};

use super::flow_connection::FlowNodeConnection;

/// Defines all node connections inside the flow
pub struct FlowNetwork {
    pub connections: HashSet<FlowNodeConnection>,
}

impl FlowNetwork {
    pub fn new() -> Self {
        FlowNetwork {
            connections: HashSet::new(),
        }
    }

    pub fn add_connection(&mut self, connection: FlowNodeConnection) {
        self.connections.insert(connection);
    }

    pub fn get_connections(&self) -> Iter<FlowNodeConnection> {
        self.connections.iter()
    }
}
