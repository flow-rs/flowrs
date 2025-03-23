#[macro_export]
macro_rules! generate_local_connection {
    ($type:ty) => {
        fn connect_nodes(
            sender_id: NodeId,
            receiver_id: NodeId,
            sender_out_idx: NodeIOIndex,
            recv_in_idx: NodeIOIndex,
            sender_io: &mut dyn Any,
            receiver_io: &mut dyn Any,
        ) {
            if let Some(sender_output) = sender_io.downcast_mut::<TypedOutput<$type>>() {
                if let Some(receiver_input) = receiver_io.downcast_mut::<TypedInput<$type>>() {
                    let (send_half, recv_half) = sender_output.split();
                    receiver_input.set_any_communicator(Box::new(recv_half));
                    sender_output.set_any_communicator(Box::new(send_half));

                    println!(
                        "[generate_local_connection] Successfully connected nodes {} -> {} with type {}",
                        sender_id, receiver_id, stringify!($type)
                    );
                } else {
                    panic!("[generate_local_connection] Receiver IO type mismatch");
                }
            } else {
                panic!("[generate_local_connection] Sender IO type mismatch");
            }
        }

        // Register the connection function
        TYPE_REGISTRY
            .lock()
            .unwrap()
            .register::<$type>(connect_nodes);

        println!(
            "[generate_local_connection] Registered connection function for type: {}",
            stringify!($type)
        );
    };
}

#[macro_export]
macro_rules! connect_nodes {
    ($type:ty, $flow:expr, $sender_id:expr, $receiver_id:expr, $sender_out_idx:expr, $recv_in_idx:expr) => {{
        generate_local_connection!($type);
        $flow.connect_nodes::<$type>($sender_id, $receiver_id, $sender_out_idx, $recv_in_idx)
    }};
}
