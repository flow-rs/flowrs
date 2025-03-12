use std::{
    any::TypeId,
    collections::{
        hash_set::{Drain as SetDrain, Iter},
        HashMap, HashSet,
    },
};

use super::flow_connection::FlowNodeConnection;

/// Defines all node connections inside the flow
pub struct FlowNetwork {
    pub connections: HashSet<FlowNodeConnection>,
    pub connection_types: HashMap<FlowNodeConnection, TypeId>,
}

impl FlowNetwork {
    pub fn new() -> Self {
        FlowNetwork {
            connections: HashSet::new(),
            connection_types: HashMap::new(),
        }
    }

    pub fn add_connection(&mut self, connection: FlowNodeConnection, type_id: TypeId) {
        self.connections.insert(connection.clone());
        self.connection_types.insert(connection, type_id);
    }
    pub fn get_connection_type(&self, connection: &FlowNodeConnection) -> Option<TypeId> {
        self.connection_types.get(connection).cloned()
    }

    pub fn get_connections(&self) -> Iter<'_, FlowNodeConnection> {
        self.connections.iter()
    }
}
