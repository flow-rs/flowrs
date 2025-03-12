use super::flow_types::{NodeIOIndex, NodeId};

/// Defines a single connection between two nodes from the flow
#[derive(Eq, Hash, PartialEq, Clone)]
pub struct FlowNodeConnection {
    // Node ID of the sending Node
    pub sender_id: NodeId,
    // Node ID of the receiving Node
    pub receiver_id: NodeId,
    // Index of the Output from the sender node
    pub send_out_idx: NodeIOIndex,
    // Index of the Input from the receiver Node
    pub recv_in_idx: NodeIOIndex,
}
