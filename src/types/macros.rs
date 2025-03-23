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

            // Downcast the sender and receiver IO to NodeIO
            let sender_node_io = sender_io
                .downcast_mut::<NodeIO<_, (TypedOutput<$type>,)>>()
                .expect("Sender node IO type mismatch");

            let receiver_node_io = receiver_io
                .downcast_mut::<NodeIO<(TypedInput<$type>,), _>>()
                .expect("Receiver node IO type mismatch");

            // Access the specific output and input at the given indices
            let sender_output = &mut (sender_node_io.outputs).0;
            let receiver_input = &mut (receiver_node_io.inputs).0;

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
