use std::{any::Any, collections::HashMap};

use crate::flow::flow_types::{NodeIOIndex, NodeId};

/// Macro to generate a connection function for local node connections
#[macro_export]
macro_rules! generate_local_connection {
    ($fn_name:ident, $type:ty) => {
        pub fn $fn_name(
            sender_id: NodeId,
            receiver_id: NodeId,
            sender_out_idx: NodeIOIndex,
            recv_in_idx: NodeIOIndex,
            sender_io: &mut dyn Any,
            receiver_io: &mut dyn Any,
        ) {
            println!(
                "[Node RT] Connecting local nodes {} -> {} (Out {} -> In {}) with type {}",
                sender_id,
                receiver_id,
                sender_out_idx,
                recv_in_idx,
                stringify!($type)
            );

            // Downcast the sender and receiver IO
            let sender_output = sender_io
                .downcast_mut::<TypedOutput<$type>>()
                .expect("Sender output type mismatch");

            let receiver_input = receiver_io
                .downcast_mut::<TypedInput<$type>>()
                .expect("Receiver input type mismatch");

            // Local handling (using thread communicators)
            let comm = sender_output
                .output
                .get_communicator_mut()
                .expect("No communicator found");
            let send_half = comm.clone_send();
            let recv_half = comm.move_recv().expect("Failed to move receiver");

            // Assign communicators
            sender_output
                .output
                .set_communicator(NodeCommunicator::ThreadComm(send_half));
            receiver_input
                .input
                .set_communicator(NodeCommunicator::ThreadComm(recv_half));

            println!(
                "[Node RT] Successfully connected local nodes {} -> {} with type {}",
                sender_id,
                receiver_id,
                stringify!($type)
            );
        }
    };
}

// Macro to register connection functions and store them in a static hashmap
lazy_static::lazy_static! {
    pub static ref LOCAL_CONNECTION_REGISTRY: std::sync::Mutex<HashMap<String, fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn Any, &mut dyn Any)>> = {
        let m = HashMap::new();
        m.into()
    };
}

#[macro_export]
macro_rules! register_local_connection {
    ($name:literal, $func:ident) => {
        LOCAL_CONNECTION_REGISTRY.lock().unwrap().insert(
            $name.to_string(),
            $func as fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn Any, &mut dyn Any),
        );
    };
}
