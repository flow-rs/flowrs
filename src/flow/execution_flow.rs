use std::any::TypeId;
use std::collections::hash_map::Drain as MapDrain;
use std::collections::hash_map::Iter as MapIter;
use std::collections::hash_set::Drain as SetDrain;
use std::collections::hash_set::Iter as SetIter;
use std::collections::HashMap;

use crate::node::Node;

use super::flow_connection::FlowNodeConnection;
use super::flow_error::FlowError;
use super::flow_network::FlowNetwork;
use super::flow_types::NodeIOIndex;
use super::flow_types::NodeId;

//use crate::connection::RuntimeNode;

pub struct ExecutionFlow {
    //nodes: Vec<(NodeId, Box<dyn RuntimeNode>)>,
    nodes: HashMap<NodeId, Box<dyn Node>>,
    network: FlowNetwork,
    // id_to_node_idx: HashMap<NodeId, usize>,
    // id_to_desc: HashMap<NodeId, NodeDescription>,
}

impl ExecutionFlow {
    pub fn new_empty() -> Self {
        Self {
            //nodes: Vec::new(),
            nodes: HashMap::new(),
            network: FlowNetwork::new(),
            // id_to_node_idx: HashMap::new(),
            // id_to_desc: HashMap::new(),
        }
    }

    //pub fn new(nodes: HashMap<NodeId, Box<dyn RuntimeNode>>) -> Self {
    //pub fn new() -> Self {
    //Self {
    //nodes: Vec::new(),
    //network: FlowNetwork::new(),
    // id_to_node_idx: HashMap::new(),
    // id_to_desc: HashMap::new(),
    // id_counter: 0,
    //}

    // for (id, runtime_node) in nodes {
    //     let node_idx = obj.nodes.len();
    //     obj.nodes.push((id, runtime_node));
    //     obj.id_to_node_idx.insert(id, node_idx);
    // }

    //obj
    //}

    // fn generate_id(&mut self) -> NodeId {
    //     self.id_counter += 1;
    //     self.id_counter
    // }

    // pub fn node_description_by_id(&self, id: NodeId) -> Option<&NodeDescription> {
    //     self.id_to_desc.get(&id)
    // }

    // pub fn add_node<T>(&mut self, node: T) -> NodeId
    // where
    //     T: RuntimeNode + 'static,
    // {
    //     let id = self.generate_id();
    //     self.add_node_with_id_and_desc(node, id, NodeDescription::default())
    // }

    pub fn add_node_with_id(&mut self, node: Box<dyn Node>, id: NodeId) -> Option<Box<dyn Node>> {
        self.nodes.insert(id, node)
    }

    // pub fn add_node_with_id_and_desc<T>(
    //     &mut self,
    //     node: T,
    //     id: NodeId,
    //     desc: NodeDescription,
    // ) -> NodeId
    // where
    //     T: RuntimeNode + 'static,
    // {
    //     if !self.id_to_node_idx.contains_key(&id) {
    //         self.nodes.push((id, Box::new(node)));
    //         self.id_to_node_idx.insert(id, self.nodes.len() - 1);
    //         self.id_to_desc.insert(id, desc);
    //     }

    //     id
    // }

    // pub fn node_by_index(&mut self, index: usize) -> Option<&mut (NodeId, Box<dyn RuntimeNode>)> {
    //     self.nodes.get_mut(index)
    // }

    // pub fn node_by_id(&mut self, id: NodeId) -> Option<&(NodeId, Box<dyn RuntimeNode>)> {
    //     if let Some(idx) = self.id_to_node_idx.get(&id) {
    //         return self.node_by_index(*idx).map(|x| &*x);
    //     }
    //     None
    // }

    // pub fn num_nodes(&self) -> usize {
    //     self.nodes.len()
    // }

    /// Inserts a node description and returns the NodeId of the inserted node
    // pub fn add_node(&mut self, node_desc: NodeDesc) -> NodeId {
    //     let id: NodeId = self.id_counter;
    //     self.id_counter = self.id_counter + 1;
    //     self.nodes.insert(id, node_desc);

    //     id
    // }

    pub fn connect_nodes(
        &mut self,
        sender_node: NodeId,
        recv_node: NodeId,
        sender_out_idx: NodeIOIndex,
        recv_in_idx: NodeIOIndex,
    ) -> Result<(), FlowError> {
        //check for errors
        match self.nodes.get(&sender_node) {
            Some(node_desc) => {
                if sender_out_idx < node_desc.get_output_count().try_into().unwrap() {
                    return Err(FlowError::InvalidNodeIOIndexError);
                }
            }
            None => return Err(FlowError::InvalidNodeIdError),
        }
        match self.nodes.get(&recv_node) {
            Some(node_desc) => {
                if recv_in_idx < node_desc.get_input_count().try_into().unwrap() {
                    return Err(FlowError::InvalidNodeIOIndexError);
                }
            }
            None => return Err(FlowError::InvalidNodeIdError),
        }
        //add connection
        let connection = FlowNodeConnection {
            sender_id: sender_node,
            receiver_id: recv_node,
            send_out_idx: sender_out_idx,
            recv_in_idx: recv_in_idx,
        };
        let type_id = self
            .network
            .get_connection_type(&connection)
            .expect("must be known");
        self.network.add_connection(connection, type_id);

        Ok(())
    }

    pub fn get_connections(&self) -> SetIter<'_, FlowNodeConnection> {
        self.network.get_connections()
    }

    pub fn set_connections(&mut self, connections: SetDrain<FlowNodeConnection>) {
        connections.for_each(|connection| {
            let type_id = self
                .network
                .get_connection_type(&connection)
                .expect("TypeId must be known before setting connections.");
            self.network.add_connection(connection, type_id);
        });
    }

    pub fn get_nodes(&self) -> MapIter<'_, NodeId, Box<dyn Node>> {
        self.nodes.iter()
    }

    /// destructive move
    pub fn move_nodes(&mut self) -> MapDrain<'_, NodeId, Box<dyn Node>> {
        self.nodes.drain()
    }

    pub fn set_nodes(&mut self, nodes: HashMap<u128, Box<dyn Node>>) {
        self.nodes = nodes;
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    // #[tracing::instrument(skip_all)]
    // pub fn init_all(&mut self) -> Result<()> {
    //     for n in &mut self.nodes {
    //         n.1.on_init()
    //             .context(format!("Unable to init node with ID {}.", n.0))?;
    //     }
    //     Ok(())
    // }

    // #[tracing::instrument(skip_all)]
    // pub fn shutdown_all(&mut self) -> Result<()> {
    //     for n in &mut self.nodes {
    //         n.1.on_shutdown()
    //             .context(format!("Unable to shutdown node with ID {}.", n.0))?;
    //     }
    //     Ok(())
    // }

    // #[tracing::instrument(skip_all)]
    // pub fn ready_all(&mut self) -> Result<()> {
    //     for n in &mut self.nodes {
    //         n.1.on_ready()
    //             .context(format!("Unable to make node with ID {}.", n.0))?;
    //     }
    //     Ok(())
    // }

    // pub fn get_update_controllers(&self) -> Vec<Box<dyn UpdateController>> {
    //     let mut update_controllers: Vec<Box<dyn UpdateController>> = Vec::new();
    //     for n in &self.nodes {
    //         if let Some(us) = n.1.update_controller() {
    //             update_controllers.push(us);
    //         }
    //     }
    //     update_controllers
    // }
}

/// Describes a Node and the number of its inputs and outputs
pub struct NodeDesc {
    node_input_count: usize,
    node_output_count: usize,
}

impl NodeDesc {
    pub fn new(input_count: usize, output_count: usize) -> Self {
        //let mut input_count = 0;
        //let mut output_count = 0;
        // if let Some(inputs) = node_type.clone().inputs {
        //     input_count = inputs.len();
        // }
        // if let Some(outputs) = node_type.clone().outputs {
        //     output_count = outputs.len();
        // }
        NodeDesc {
            node_input_count: input_count,
            node_output_count: output_count,
        }
    }
}
