#[macro_export]
macro_rules! generate_local_connection {
    ($type:ty) => {
        paste::item! {
            pub fn [<connect_nodes_ $type>](sender_id: NodeId, receiver_id: NodeId, sender_out_idx: NodeIOIndex, recv_in_idx: NodeIOIndex, sender_io: &mut dyn Any, receiver_io: &mut dyn Any) {
                println!("[generate_local_connection] Attempting to connect nodes with type: {:?}", std::any::TypeId::of::<$type>());

                // Retrieve the sender output
                if let Some(typed_sender_io) = sender_io.downcast_mut::<TypedOutput<$type>>() {
                    println!("[generate_local_connection] Successfully downcasted sender IO to TypedOutput<{}>", stringify!($type));

                    let (send_half, recv_half) = typed_sender_io.split(sender_out_idx);

                    // Set sender communicator
                    typed_sender_io.set_any_communicator(Box::new(send_half) as Box<dyn Any + Send>);

                    // Retrieve the receiver input
                    if let Some(typed_receiver_io) = receiver_io.downcast_mut::<TypedInput<$type>>() {
                        println!("[generate_local_connection] Successfully downcasted receiver IO to TypedInput<{}>", stringify!($type));
                        typed_receiver_io.set_any_communicator(Box::new(recv_half) as Box<dyn Any + Send>);
                        println!("[generate_local_connection] Successfully connected nodes: {} -> {}", sender_id, receiver_id);
                    } else {
                        panic!("[generate_local_connection] Receiver IO type mismatch");
                    }
                } else {
                    panic!("[generate_local_connection] Sender IO type mismatch");
                }
            }
        }
    };
}

#[macro_export]
macro_rules! connect_nodes {
    ($type:ty, $flow:ident, $sender_id:expr, $receiver_id:expr, $send_idx:expr, $recv_idx:expr) => {{
        use flowrs::flow::flow_types::{NodeIOIndex, NodeId};
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

        // Register the connection function in the type registry
        TYPE_REGISTRY.lock().unwrap().register::<$type>(connect_fn);

        // Add the connection to the abstract flow
        $flow.connect_nodes::<$type>($sender_id, $receiver_id, $send_idx, $recv_idx)
    }};
}
