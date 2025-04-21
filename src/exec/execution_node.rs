use anyhow::anyhow;
use std::{any::TypeId, collections::HashMap};
use tokio::time::sleep;
use tokio::time::Duration;

use crate::node::InitError;
use crate::node::ReadyError;
use crate::node::ShutdownError;
use crate::nodes::node_io::SetupIO;
use crate::{
    comm::messages::Message,
    connection::Edge,
    flow::flow_types::NodeIOIndex,
    node::{Node, ReceiveError, UpdateError},
    types::type_registry::POLL_REGISTRY,
};

use super::{execution_mode::ExecutionMode, execution_state::ExecutionState};

pub struct ExecutionNode {
    execution_mode: ExecutionMode,
    execution_state: ExecutionState,
    pub node: Box<dyn Node>,
    control_edge: Edge<String>,
    input_type_ids: HashMap<NodeIOIndex, TypeId>,
}

impl ExecutionNode {
    pub fn new(
        node: Box<dyn Node + Send + Sync>,
        execution_mode: ExecutionMode,
        control_edge: Edge<String>,
        input_type_ids: HashMap<NodeIOIndex, TypeId>,
    ) -> Self {
        ExecutionNode {
            node,
            execution_mode,
            execution_state: ExecutionState::Initialized,
            control_edge,
            input_type_ids,
        }
    }

    pub fn on_message(&mut self, msg: Message<String>) {
        match msg {
            // Handle Peer Connection Messages
            Message::RequestPeerConnection(sender, receiver, out_idx, in_idx, dtype) => {
                println!(
                    "[ExecutionNode] Received Peer Connection Request: {} -> {} (Out {} -> In {})",
                    sender, receiver, out_idx, in_idx
                );

                // TODO: Implement logic to handle connection request.
            }
            Message::AcceptPeerConnection(sender, receiver, out_idx, in_idx, port) => {
                println!(
                    "[ExecutionNode] Peer Connection Accepted: {}(idx: {}) -> {}(idx: {}) on Port {}",
                    sender, receiver,out_idx, in_idx, port
                );

                // TODO: Implement logic to finalize accepted connection.
            }
            Message::RejectPeerConnection(sender, receiver, out_idx, in_idx, reason) => {
                println!(
                    "[ExecutionNode] Peer Connection Rejected: {}(idx: {}) -> {}(idx: {}) | Reason: {}",
                    sender, receiver, out_idx, in_idx, reason
                );

                // TODO: Handle rejected connections appropriately.
            }
            // Ignore all other messages
            _ => {
                println!("[ExecutionNode] Ignoring irrelevant message: {:?}", msg);
            }
        }
    }

    pub async fn on_update_async(&mut self) -> Result<(), UpdateError> {
        match self.execution_state {
            ExecutionState::Ready => {
                self.execution_state = ExecutionState::Running;

                println!(
                    "[ExecutionNode] Entered on_update_async() | Mode: {:?}, State: {:?}",
                    self.execution_mode, self.execution_state
                );

                let mut result = Ok(());

                loop {
                    println!(
                        "\n[ExecutionNode] Loop tick for node... State: {:?}",
                        self.execution_state
                    );

                    // Shutdown check
                    if self.execution_state == ExecutionState::Shutdown {
                        println!("[ExecutionNode] Shutdown triggered.");
                        if let Err(e) = self.on_shutdown() {
                            println!("[WARN] Shutdown failed: {}", e);
                        }
                        break;
                    }

                    // Check for control messages
                    match self.control_edge.try_message().await {
                        Ok(Some(msg)) => {
                            println!("[ExecutionNode] Control message received: {:?}", msg);
                            self.on_message(msg);
                        }
                        Ok(None) | Err(ReceiveError::NoMessageAvailable) => {
                            println!("[ExecutionNode] No control message available.");
                        }
                        Err(ReceiveError::ControlMessage(msg)) => {
                            println!("[ExecutionNode] Control message error: {:?}", msg);
                            return Err(UpdateError::ControlMessage(msg));
                        }
                        Err(ReceiveError::Other(e)) => {
                            println!("[ExecutionNode] Receive error: {}", e);
                            return Err(UpdateError::RecvError {
                                message: e.to_string(),
                            });
                        }
                    }

                    // Poll all inputs only if they don't already have buffered data
                    println!("[ExecutionNode] Polling all inputs...");
                    let io = self.node.get_io_mut();
                    let mut registry = POLL_REGISTRY.lock().await;

                    for (idx, type_id) in &self.input_type_ids {
                        // Check if buffer is already filled
                        if io.has_ready_input(*idx) {
                            continue;
                        }

                        if let Some(poll_fn) = registry.get_mut(&type_id) {
                            match poll_fn.poll(io).await {
                                Ok(_) => (),
                                Err(ReceiveError::ControlMessage(msg)) => {
                                    return Err(UpdateError::ControlMessage(msg));
                                }
                                Err(e) => {
                                    return Err(UpdateError::RecvError {
                                        message: e.to_string(),
                                    });
                                }
                            }
                        } else {
                            println!(
                            "[ExecutionNode] No PollFn registered for TypeId {:?} (input idx: {:?})",
                            type_id, idx
                        );
                        }
                    }

                    println!("[ExecutionNode] Input polling completed.");

                    // Check if all inputs are ready before calling on_update()
                    let io = self.node.get_io_mut();
                    let all_inputs_ready = self
                        .input_type_ids
                        .keys()
                        .all(|idx| io.has_ready_input(*idx));

                    if !all_inputs_ready {
                        println!(
                            "[ExecutionNode] Skipping on_update() — not all inputs ready: {:?}",
                            self.input_type_ids
                                .keys()
                                .filter(|idx| !io.has_ready_input(**idx))
                                .collect::<Vec<_>>()
                        );
                    } else {
                        println!("[ExecutionNode] ⚙ Calling node.on_update()...");
                        result = self.node.on_update();

                        match result {
                            Ok(_) => {
                                println!("[ExecutionNode] Node logic executed successfully.")
                            }
                            Err(ref e) => println!("[ExecutionNode] Node logic error: {:?}", e),
                        }
                    }

                    // Handle execution mode
                    match self.execution_mode {
                        ExecutionMode::Synchronized => {
                            println!("[ExecutionNode] Exiting loop (Synchronized mode)");
                            self.execution_state = ExecutionState::Ready;
                            break;
                        }
                        ExecutionMode::Continuous => {
                            println!("[ExecutionNode] Looping again after delay...");
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }

                result
            }

            ExecutionState::Sleeping => Err(UpdateError::AlreadyRunningError {
                message: "The node is Sleeping".to_string(),
            }),
            ExecutionState::Running => Err(UpdateError::AlreadyRunningError {
                message: "The node is Running".to_string(),
            }),
            ExecutionState::Initialized => Err(UpdateError::NotReadyError {
                message: "The node is not ready".to_string(),
            }),
            ExecutionState::Shutdown => Ok(()),
        }
    }
}

impl Node for ExecutionNode {
    fn set_execution_mode(&mut self, mode: ExecutionMode) -> ExecutionMode {
        self.execution_mode = mode.clone();
        mode
    }

    fn on_update(&mut self) -> Result<(), UpdateError> {
        Err(UpdateError::Other(anyhow!(
            "ExecutionNode requires async context.".to_string(),
        )))
    }

    fn get_execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Continuous
    }

    fn on_init(&mut self) -> Result<(), InitError> {
        Ok(())
    }

    fn on_ready(&mut self) -> Result<(), ReadyError> {
        if self.execution_state == ExecutionState::Initialized {
            println!("[ExecutionNode] Node is now READY.");
            self.execution_state = ExecutionState::Ready;
            Ok(())
        } else {
            Err(ReadyError::from(
                format!(
                    "Node is not in an initialized state: {}",
                    self.execution_state,
                )
                .as_str(),
            ))
        }
    }

    fn on_shutdown(&mut self) -> Result<(), ShutdownError> {
        Ok(())
    }

    fn get_input_count(&self) -> u128 {
        self.node.get_input_count()
    }

    fn get_output_count(&self) -> u128 {
        self.node.get_output_count()
    }

    fn setup_input(&mut self, idx: u128, local: bool) {
        self.node.setup_input(idx, local);
    }

    fn setup_output(&mut self, idx: u128, local: bool) {
        self.node.setup_output(idx, local);
    }

    fn get_io_mut(&mut self) -> &mut dyn SetupIO {
        self.node.get_io_mut() // Now correctly returns `&mut dyn SetupIO`
    }
}
