use crate::exec::execution_directive::NodeExecutionDirective;
use crate::flow::flow_types::NodeId;
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
    node_id: NodeId,
    control_edge: Edge<String>,
    input_type_ids: HashMap<NodeIOIndex, TypeId>,
}

impl ExecutionNode {
    pub fn new(
        node: Box<dyn Node + Send + Sync>,
        node_id: NodeId,
        execution_mode: ExecutionMode,
        control_edge: Edge<String>,
        input_type_ids: HashMap<NodeIOIndex, TypeId>,
    ) -> Self {
        ExecutionNode {
            node,
            node_id,
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
        self.execution_state = ExecutionState::Running;

        println!(
            "[ExecutionNode] Node {} entered on_update_async() | Mode: {:?}, State: {:?}",
            self.node_id, self.execution_mode, self.execution_state
        );

        let mut result = Ok(());

        loop {
            println!(
                "\n[ExecutionNode] Loop tick for Node {} State: {:?}",
                self.node_id, self.execution_state
            );

            if self.execution_state == ExecutionState::Shutdown {
                println!("[ExecutionNode] Node {}: Shutdown triggered.", self.node_id);
                if let Err(e) = self.on_shutdown() {
                    println!("[WARN] Node {}: Shutdown failed: {}", self.node_id, e);
                }
                break;
            }

            match self.control_edge.try_message().await {
                Ok(Some(msg)) => {
                    println!(
                        "[ExecutionNode] Node {} received Control Message: {:?}",
                        self.node_id, msg
                    );
                    self.on_message(msg);
                }
                Ok(None) | Err(ReceiveError::NoMessageAvailable) => {
                    println!(
                        "[ExecutionNode] Node {}: No control message available.",
                        self.node_id
                    );
                }
                Err(ReceiveError::ControlMessage(msg)) => {
                    println!(
                        "[ExecutionNode] Node {}: Control message error: {:?}",
                        self.node_id, msg
                    );
                    return Err(UpdateError::ControlMessage(msg));
                }
                Err(ReceiveError::Other(e)) => {
                    println!(
                        "[ExecutionNode] Node {}: Receive error: {}",
                        self.node_id, e
                    );
                    return Err(UpdateError::RecvError {
                        message: e.to_string(),
                    });
                }
            }

            let directive = self.node.on_update_directive()?;
            println!(
                "[ExecutionNode] Node {}: on_update_directive → {:?}",
                self.node_id, directive
            );

            let required_inputs = match directive {
                NodeExecutionDirective::ContinueImmediately => None,
                NodeExecutionDirective::Suspend => {
                    println!("[ExecutionNode] Node {}: Suspended.", self.node_id);
                    self.execution_state = ExecutionState::Sleeping;
                    break;
                }
                NodeExecutionDirective::WaitForInputs(req) => Some(req),
            };

            let io = self.node.get_io_mut();
            let mut registry = POLL_REGISTRY.lock().await;

            if let Some(inputs) = required_inputs.as_ref() {
                println!(
                    "[ExecutionNode] Node {}: Polling required inputs: {:?}",
                    self.node_id, inputs
                );

                for idx in inputs {
                    let already_ready = io.has_ready_input(*idx);
                    println!(
                        "[ExecutionNode] Node {}: Input {} buffer ready? {}",
                        self.node_id, idx, already_ready
                    );

                    if already_ready {
                        continue;
                    }

                    if let Some(type_id) = self.input_type_ids.get(idx) {
                        if let Some(poll_fn) = registry.get_mut(type_id) {
                            println!(
                                "[ExecutionNode] Node {}: Polling input {:?}...",
                                self.node_id, idx
                            );
                            match poll_fn.poll(io, *idx).await {
                                Ok(_) => println!(
                                    "[ExecutionNode] Node {}: Polled input {} (TypeId: {:?})",
                                    self.node_id, idx, type_id
                                ),
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
                            "[ExecutionNode] Node {}: ❌ No PollFn registered for TypeId {:?} at index {}",
                            self.node_id, type_id, idx
                        );
                        }
                    }
                }
            }

            println!(
                "[ExecutionNode] Node {}: Input polling completed.",
                self.node_id
            );

            let all_ready = required_inputs
                .as_ref()
                .map(|inputs| inputs.iter().all(|i| io.has_ready_input(*i)))
                .unwrap_or(true);

            if all_ready {
                println!(
                    "[ExecutionNode] Node {}: Calling node.on_update()...",
                    self.node_id
                );
                result = self.node.on_update();
                match result {
                    Ok(_) => println!(
                        "[ExecutionNode] Node {}: ✅ Node logic executed successfully.",
                        self.node_id
                    ),
                    Err(ref e) => println!(
                        "[ExecutionNode] Node {}: ❌ Node logic error: {:?}",
                        self.node_id, e
                    ),
                }
            } else {
                println!(
                    "[ExecutionNode] Node {}: ⏭ Skipping on_update(), missing inputs: {:?}",
                    self.node_id,
                    required_inputs
                        .unwrap()
                        .iter()
                        .filter(|idx| !io.has_ready_input(**idx))
                        .collect::<Vec<_>>()
                );
            }

            match self.execution_mode {
                ExecutionMode::Synchronized => {
                    println!(
                        "[ExecutionNode] Node {}: Exiting loop (Synchronized mode)",
                        self.node_id
                    );
                    self.execution_state = ExecutionState::Ready;
                    break;
                }
                ExecutionMode::Continuous => {
                    println!(
                        "[ExecutionNode] Node {}: Sleeping before next tick...",
                        self.node_id
                    );
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
            }
        }

        result
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
