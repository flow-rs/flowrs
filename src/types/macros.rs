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

macro_rules! connect_nodes {
    ($type:ty, $flow:expr, $sender_id:expr, $receiver_id:expr, $sender_out_idx:expr, $recv_in_idx:expr) => {{
        // Generate the local connection function for the given type
        generate_local_connection!($type);

        // Register the connection function in the registry
        TYPE_REGISTRY.lock().unwrap().register::<$type>(Box::new([<connect_nodes_ $type>]));

        // Create the abstract connection in the flow
        $flow.connect_nodes::<$type>($sender_id, $receiver_id, $sender_out_idx, $recv_in_idx)
    }};
}
