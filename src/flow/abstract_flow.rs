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

pub struct AbstractFlow {
    nodes: HashMap<NodeId, Box<dyn Node>>,
    network: FlowNetwork,
}

impl AbstractFlow {
    pub fn new_empty() -> Self {
        Self {
            //nodes: Vec::new(),
            nodes: HashMap::new(),
            network: FlowNetwork::new(),
        }
    }

    pub fn add_node_with_id(&mut self, node: Box<dyn Node>, id: NodeId) -> Option<Box<dyn Node>> {
        self.nodes.insert(id, node)
    }

    pub fn connect_nodes(
        &mut self,
        sender_node: NodeId,
        recv_node: NodeId,
        sender_out_idx: NodeIOIndex,
        recv_in_idx: NodeIOIndex,
    ) -> Result<(), FlowError> {
        //check for errors
        match self.nodes.get(&sender_node) {
            Some(node) => {
                if sender_out_idx >= node.get_output_count() {
                    return Err(FlowError::InvalidNodeIOIndexError);
                }
            }
            None => return Err(FlowError::InvalidNodeIdError),
        }
        match self.nodes.get(&recv_node) {
            Some(node) => {
                if recv_in_idx >= node.get_input_count() {
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
        self.network.add_connection(connection);

        Ok(())
    }

    pub fn get_connections(&self) -> SetIter<FlowNodeConnection> {
        self.network.get_connections()
    }

    /// destructive move
    pub fn move_connections(&mut self) -> SetDrain<'_, FlowNodeConnection> {
        self.network.connections.drain()
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
        NodeDesc {
            node_input_count: input_count,
            node_output_count: output_count,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{abstract_flow::AbstractFlow, flow_error::FlowError};
    /// A mock node for testing, simulating a basic node with configurable input/output counts.
    pub struct MockNode {
        input_count: u128,
        output_count: u128,
    }

    impl MockNode {
        /// Creates a new MockNode with given input and output ports.
        pub fn new(input_count: u128, output_count: u128) -> Self {
            MockNode {
                input_count,
                output_count,
            }
        }
    }

    impl Node for MockNode {
        fn get_input_count(&self) -> u128 {
            self.input_count
        }

        fn get_output_count(&self) -> u128 {
            self.output_count
        }

        fn setup_input(&mut self, _idx: u128, _local: bool) {
            //
        }

        fn setup_output(&mut self, _idx: u128, _local: bool) {
            //
        }
    }

    /// Helper function to create a test node
    fn create_test_node() -> Box<dyn Node> {
        Box::new(MockNode::new(2, 3))
    }

    #[test]
    fn test_new_empty_flow() {
        let flow = AbstractFlow::new_empty();
        assert_eq!(flow.num_nodes(), 0, "New flow should have zero nodes");
    }

    #[test]
    fn test_add_node_with_id() {
        let mut flow = AbstractFlow::new_empty();
        let node_id = 1;
        let node = create_test_node();

        assert!(
            flow.add_node_with_id(node, node_id).is_none(),
            "Node should be added successfully"
        );
        assert_eq!(
            flow.num_nodes(),
            1,
            "Flow should have one node after addition"
        );
    }

    #[test]
    fn test_add_duplicate_node() {
        let mut flow = AbstractFlow::new_empty();
        let node_id = 1;
        let node1 = create_test_node();
        let node2 = create_test_node();

        assert!(
            flow.add_node_with_id(node1, node_id).is_none(),
            "First node should be added successfully"
        );
        assert!(
            flow.add_node_with_id(node2, node_id).is_some(),
            "Second node with the same ID should return the old node"
        );
        assert_eq!(
            flow.num_nodes(),
            1,
            "Flow should still have one node after replacement"
        );
    }

    #[test]
    fn test_connect_nodes_success() {
        let mut flow = AbstractFlow::new_empty();
        let node1 = create_test_node();
        let node2 = create_test_node();
        let id1 = 1;
        let id2 = 2;

        flow.add_node_with_id(node1, id1);
        flow.add_node_with_id(node2, id2);

        let result = flow.connect_nodes(id1, id2, 0, 0);
        assert!(result.is_ok(), "Nodes should connect successfully");
        assert_eq!(
            flow.get_connections().count(),
            1,
            "There should be one connection"
        );
    }

    #[test]
    fn test_connect_nodes_invalid_ids() {
        let mut flow = AbstractFlow::new_empty();
        let node1 = create_test_node();
        let id1 = 1;
        let id_invalid = 999; // Non-existing node ID

        flow.add_node_with_id(node1, id1);

        let result = flow.connect_nodes(id1, id_invalid, 0, 0);
        assert!(
            matches!(result, Err(FlowError::InvalidNodeIdError)),
            "Should return an InvalidNodeIdError"
        );
    }

    #[test]
    fn test_connect_nodes_invalid_io_index() {
        let mut flow = AbstractFlow::new_empty();
        let node1 = create_test_node();
        let node2 = create_test_node();
        let id1 = 1;
        let id2 = 2;

        flow.add_node_with_id(node1, id1);
        flow.add_node_with_id(node2, id2);

        let result = flow.connect_nodes(id1, id2, 10, 0); // Invalid output index
        assert!(
            matches!(result, Err(FlowError::InvalidNodeIOIndexError)),
            "Should return an InvalidNodeIOIndexError"
        );
    }

    #[test]
    fn test_get_nodes() {
        let mut flow = AbstractFlow::new_empty();
        let node1 = create_test_node();
        let node2 = create_test_node();

        flow.add_node_with_id(node1, 1);
        flow.add_node_with_id(node2, 2);

        assert_eq!(flow.get_nodes().count(), 2, "Flow should contain two nodes");
    }

    #[test]
    fn test_move_nodes() {
        let mut flow = AbstractFlow::new_empty();
        flow.add_node_with_id(create_test_node(), 1);
        flow.add_node_with_id(create_test_node(), 2);

        let moved_nodes: Vec<_> = flow.move_nodes().collect();
        assert_eq!(moved_nodes.len(), 2, "Should have moved two nodes");
        assert_eq!(
            flow.num_nodes(),
            0,
            "Flow should be empty after moving nodes"
        );
    }

    #[test]
    fn test_move_connections() {
        let mut flow = AbstractFlow::new_empty();
        let id1 = 1;
        let id2 = 2;
        flow.add_node_with_id(create_test_node(), id1);
        flow.add_node_with_id(create_test_node(), id2);
        flow.connect_nodes(id1, id2, 0, 0).unwrap();

        let moved_connections: Vec<_> = flow.move_connections().collect();
        assert_eq!(
            moved_connections.len(),
            1,
            "Should have moved one connection"
        );
        assert_eq!(
            flow.get_connections().count(),
            0,
            "Flow should have no connections after move"
        );
    }
}
