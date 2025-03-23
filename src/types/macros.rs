#[macro_export]
macro_rules! generate_local_connection {
    ($type:ty) => {
        // Generate a top-level function for connecting nodes
        #[allow(non_snake_case)]
        pub fn connect_nodes_$type(
            sender_id: NodeId,
            receiver_id: NodeId,
            sender_out_idx: NodeIOIndex,
            recv_in_idx: NodeIOIndex,
            sender_io: &mut dyn Any,
            receiver_io: &mut dyn Any,
        ) {
            if let Some(sender_output) = sender_io.downcast_mut::<TypedOutput<$type>>() {
                if let Some(receiver_input) = receiver_io.downcast_mut::<TypedInput<$type>>() {
                    let (send_half, recv_half) = sender_output.split(sender_out_idx);
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

        // Register the connection function in the type registry
        lazy_static::lazy_static! {
            static ref REGISTER: () = {
                let func: ConnectionFn = connect_nodes_$type;
                TYPE_REGISTRY.lock().unwrap().register::<$type>(func);
            };
        }
    };
}

/// Macro to generate connection functions and handle registration in the type registry
#[macro_export]
macro_rules! connect_nodes {
    ($type:ty, $flow:expr, $sender_id:expr, $receiver_id:expr, $sender_out_idx:expr, $recv_in_idx:expr) => {{
        paste::paste! {
            // Generate the local connection function for the given type
            generate_local_connection!($type);

            // Register the connection function in the registry
            TYPE_REGISTRY.lock().unwrap().register::<$type>(Box::new([<connect_nodes_ $type>]));

            // Create the abstract connection in the flow
            $flow.connect_nodes::<$type>($sender_id, $receiver_id, $sender_out_idx, $recv_in_idx)
        }
    }};
}
