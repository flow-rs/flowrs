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

            // Attempt to downcast to a generic SetupIO trait object
            let sender_io = sender_io
                .downcast_mut::<dyn SetupIO>()
                .expect("Sender IO type mismatch");

            let receiver_io = receiver_io
                .downcast_mut::<dyn SetupIO>()
                .expect("Receiver IO type mismatch");

            // Split the communicator from the sender side
            let (send_half, recv_half) = sender_io
                .split(sender_out_idx)
                .expect("Failed to split communicator");

            // Set the communicator on the sender output
            sender_io
                .set_any_communicator(sender_out_idx, send_half)
                .expect("Failed to set sender communicator");

            // Set the communicator on the receiver input
            receiver_io
                .set_any_communicator(recv_in_idx, recv_half)
                .expect("Failed to set receiver communicator");

            println!(
                "[Node RT] Successfully connected local nodes {} -> {} with type {}",
                sender_id,
                receiver_id,
                stringify!($type)
            );
        }
    };
}

#[macro_export]
macro_rules! connect_nodes {
    ($type:ty, $flow:expr, $sender_id:expr, $recv_id:expr, $sender_out_idx:expr, $recv_in_idx:expr) => {{
        // Generate the connection function
        generate_local_connection!(connect_fn, $type);

        // Register the function in the TypeRegistry
        unsafe {
            if let Some(mut reg) = TYPE_REGISTRY.lock().ok() {
                reg.register::<$type>(connect_fn);
                println!(
                    "[DEBUG] Registered connection function for type: {}",
                    stringify!($type)
                );
            } else {
                panic!("[ERROR] Failed to acquire lock on TYPE_REGISTRY");
            }
        }

        // Add the abstract connection to the flow
        $flow.connect_nodes::<$type>($sender_id, $recv_id, $sender_out_idx, $recv_in_idx)
    }};
}
