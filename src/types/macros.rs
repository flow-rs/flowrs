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
