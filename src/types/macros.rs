#[macro_export]
macro_rules! generate_local_connection {
    ($type:ty, $sender_id:expr, $receiver_id:expr, $send_idx:expr, $recv_idx:expr, $sender_io:expr, $receiver_io:expr) => {{
        use flowrs::nodes::node_io::{SettableCommunicator, SplittableCommunicator};
        use std::any::Any;

        // Step 1: Downcast the sender and receiver IO to the correct types
        let sender_io = $sender_io
            .downcast_mut::<TypedOutput<$type>>()
            .expect("[generate_local_connection] Sender IO type mismatch");
        let receiver_io = $receiver_io
            .downcast_mut::<TypedInput<$type>>()
            .expect("[generate_local_connection] Receiver IO type mismatch");

        // Step 2: Get the communicator from the sender and split it
        let (send_half, recv_half) = sender_io.split();

        // Step 3: Set the communicator halves on the sender and receiver
        sender_io.set_any_communicator(send_half);
        receiver_io.set_any_communicator(recv_half);

        println!(
            "[DEBUG] Successfully connected local nodes {} -> {} with type {}",
            $sender_id,
            $receiver_id,
            stringify!($type)
        );
    }};
}

#[macro_export]
macro_rules! connect_nodes {
    ($type:ty, $flow:ident, $sender_id:expr, $receiver_id:expr, $send_idx:expr, $recv_idx:expr) => {{
        use flowrs::types::type_registry::TYPE_REGISTRY;
        use std::any::{Any, TypeId};

        // Register the type ID and the connection function in the registry
        let type_id = TypeId::of::<$type>();

        let connect_fn: fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn Any, &mut dyn Any) =
            |sender_id, receiver_id, send_idx, recv_idx, sender_io, receiver_io| {
                generate_local_connection!(
                    $type,
                    sender_id,
                    receiver_id,
                    send_idx,
                    recv_idx,
                    sender_io,
                    receiver_io
                );
            };

        // Insert the function into the registry
        TYPE_REGISTRY.lock().unwrap().register(type_id, connect_fn);

        // Add the connection to the abstract flow
        $flow.connect_nodes::<$type>($sender_id, $receiver_id, $send_idx, $recv_idx)
    }};
}
