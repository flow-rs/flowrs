#[macro_export]
macro_rules! generate_local_connection {
    ($type:ty) => {
        fn connect_nodes(
            sender_id: NodeId,
            receiver_id: NodeId,
            sender_out_idx: NodeIOIndex,
            recv_in_idx: NodeIOIndex,
            sender_io: &mut dyn SetupIO,
            receiver_io: &mut dyn SetupIO,
        ) {
            tracing::debug!(
                "[DEBUG] Registering connection function for type ID: {:?} (type: {})",
                TypeId::of::<$type>(),
                stringify!($type)
            );

            if let Some(sender_output_any) = sender_io.get_output_communicator(sender_out_idx) {
                if let Some(sender_output_wrapper) = sender_output_any.downcast_mut::<TypedOutput<$type>>() {
                    let sender_output = &mut sender_output_wrapper.output;
                    if let Some(receiver_input_any) = receiver_io.get_input_communicator(recv_in_idx) {
                        if let Some(receiver_input_wrapper) = receiver_input_any.downcast_mut::<TypedInput<$type>>() {
                            let receiver_input = &mut receiver_input_wrapper.input;
                            if let Some(existing_comm) = sender_output.get_communicator_mut() {
                                let send_half = existing_comm.clone_send();
                                let recv_half = existing_comm.move_recv().expect("Failed to move receiver");

                                sender_output.set_communicator(NodeCommunicator::ThreadComm(send_half));
                                receiver_input.set_communicator(NodeCommunicator::ThreadComm(recv_half));

                                tracing::debug!(
                                    "[connect_nodes] Successfully connected nodes {} -> {} with type {}",
                                    sender_id,
                                    receiver_id,
                                    stringify!($type)
                                );
                            } else {
                                panic!("[connect_nodes] No communicator to split!");
                            }
                        } else {
                            panic!("[connect_nodes] Receiver IO type mismatch");
                        }
                    } else {
                        panic!("[connect_nodes] Failed to get input communicator");
                    }
                } else {
                    panic!("[connect_nodes] Sender IO type mismatch");
                }
            } else {
                panic!("[connect_nodes] Failed to get output communicator");
            }
        }

        // Register both local and dynamic factory/setup functions
        let mut registry = TYPE_REGISTRY.lock().await;
        registry.register::<$type>(connect_nodes);                    // local connection
        #[cfg(not(target_arch = "wasm32"))]
        registry.register_communicator::<$type>(stringify!($type));  // P2P factory + IO setup

         // Register polling function in the separate registry
        let poll_fn: PollFn<$type> = Box::new(|io, idx| {
            Box::pin(async move {
                if let Some(edge_any) = io.get_input_communicator(idx) {
                    let typed_input = edge_any
                        .downcast_mut::<TypedInput<$type>>()
                        .ok_or_else(|| ReceiveError::<$type>::Other(anyhow::anyhow!(
                            "Downcast to TypedInput<{}> failed at index {}",
                            stringify!($type),
                            idx
                        )))?;

                    typed_input.input.edge.poll_and_buffer().await?;
                } else {
                    return Err(ReceiveError::<$type>::Other(anyhow::anyhow!(
                        "No input communicator found at index {}",
                        idx
                    )));
                }

                Ok(())
            })
        });


        let mut poll_registry = POLL_REGISTRY.lock().await;
        poll_registry.register_poll_fn::<$type>(poll_fn);

        tracing::debug!(
            "[generate_local_connection] Fully registered type: {}",
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
