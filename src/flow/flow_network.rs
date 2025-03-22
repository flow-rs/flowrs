use std::{
    any::TypeId,
    collections::{
        hash_set::{Drain as SetDrain, Iter},
        HashMap, HashSet,
    },
};

use super::{
    flow_connection::FlowNodeConnection,
    flow_types::{NodeIOIndex, NodeId},
};

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

    fn get_sender_connections(&self, sender_node: NodeId) -> HashSet<FlowNodeConnection> {
        let connections = self.get_connections();
        connections
            .filter(|con| con.sender_id == sender_node)
            .cloned()
            .collect()
    }

    fn get_receiver_connections(&self, receiver_node: NodeId) -> HashSet<FlowNodeConnection> {
        let connections = self.get_connections();
        connections
            .filter(|con| con.receiver_id == receiver_node)
            .cloned()
            .collect()
    }

    pub fn get_output_type(
        &self,
        sender_node: NodeId,
        sender_output_idx: NodeIOIndex,
    ) -> Option<TypeId> {
        let sender_connections: HashSet<FlowNodeConnection> = self
            .get_sender_connections(sender_node)
            .iter()
            .filter(|con| (*con).send_out_idx == sender_output_idx)
            .cloned()
            .collect();

        if sender_connections.len() != 1 {
            println!("[DEBUG] found connections: {:?}", sender_connections);
            println!("[FLOW NETWORK ERROR] wrong amount of connections found!");
            None
        } else {
            let single_con = sender_connections.into_iter().next().unwrap();
            self.get_connection_type(&single_con)
        }
    }

    pub fn get_input_type(
        &self,
        receiver_node: NodeId,
        receiver_input_idx: NodeIOIndex,
    ) -> Option<TypeId> {
        let receiver_connections: HashSet<FlowNodeConnection> = self
            .get_receiver_connections(receiver_node)
            .iter()
            .filter(|con| (*con).recv_in_idx == receiver_input_idx)
            .cloned()
            .collect();

        println!("[DEBUG] found connections: {:?}", receiver_connections);

        if receiver_connections.len() != 1 {
            println!("[DEBUG] found connections: {:?}", receiver_connections);
            println!("[FLOW NETWORK ERROR] wrong amount of connections found!");
            None
        } else {
            let single_con = receiver_connections.into_iter().next().unwrap();
            self.get_connection_type(&single_con)
        }
    }
}
