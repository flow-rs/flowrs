use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::comm::communication::NodeCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use crate::connection::Edge;
use crate::exec::execution_node::ExecutionNode;
use crate::flow::abstract_flow::AbstractFlow;
use crate::flow::flow_types::{NodeIOIndex, NodeId};
use crate::node::Node;
use crate::scheduler::Scheduler;

use super::execution_configuration::{ExecutionConfig, NodeConfig};
use super::execution_mode::ExecutionMode;

pub struct ExecutionContext {
    pub executor: StandardExecutor,
    pub flow: AbstractFlow,
}

impl ExecutionContext {
    pub fn new(executor: StandardExecutor, flow: AbstractFlow) -> Self {
        Self {
            executor: executor,
            flow: flow,
        }
    }
}

pub trait Executor {
    fn run<S, U>(&mut self, flow: AbstractFlow, scheduler: S, node_updater: U) -> Result<()>
    where
        S: Scheduler + std::marker::Send;
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Node Setup Failed Error. Message: {message:?}")]
    NodeSetupFailed { message: String },

    #[error("Node Communication Setup Failed Error. Message: {message:?}")]
    CommunicationSetupFailed { message: String },
}

pub struct StandardExecutor {
    pub execution_mode: ExecutionMode,
    pub execution_nodes: HashMap<NodeId, Arc<Mutex<ExecutionNode>>>,
}

impl StandardExecutor {
    pub fn new() -> Self {
        Self {
            execution_mode: ExecutionMode::Continuous,
            execution_nodes: HashMap::new(),
        }
    }

    pub fn has_node(&self, node_id: NodeId) -> bool {
        self.execution_nodes.contains_key(&node_id)
    }

    /// **Creates Execution Nodes for each local node in the flow.**
    pub async fn initialize_nodes(
        &mut self,
        abstract_flow: Arc<Mutex<AbstractFlow>>,
        execution_config: &ExecutionConfig,
    ) -> Result<(), ExecutionError> {
        tracing::info!("[Executor] Initializing local nodes...");

        let mut initialized_nodes = HashMap::new();

        // Lock the flow and drain the nodes into a temporary vector
        let drained_nodes = {
            let mut flow_guard = abstract_flow.lock().await;
            flow_guard.move_nodes().collect::<Vec<_>>() // collect drops the drain (mutable borrow)
        };

        // Re-lock for read access now that the mutable borrow is gone
        let flow_guard = abstract_flow.lock().await;

        tracing::info!(
            "[Executor] Retrieved {} nodes from flow",
            drained_nodes.len()
        );

        for (node_id, node) in drained_nodes {
            match execution_config.node_configs.get(&node_id) {
                Some(NodeConfig::LocalNodeConfig) => {
                    tracing::debug!("[Executor] Creating execution node for ID: {}", node_id);

                    // Collect input type IDs for this node
                    let mut input_type_ids = HashMap::new();
                    for conn in flow_guard.get_connections() {
                        if conn.receiver_id == node_id {
                            if let Some((_, type_id)) = flow_guard.get_connection_type(conn) {
                                input_type_ids.insert(conn.recv_in_idx, type_id);
                            } else {
                                return Err(ExecutionError::NodeSetupFailed {
                                    message: format!(
                                        "Missing type ID for input of node {}",
                                        node_id
                                    ),
                                });
                            }
                        }
                    }

                    // Create the node (now synchronous, no await)
                    match self.create_local_execution_node(
                        node,
                        node_id,
                        self.execution_mode.clone(),
                        input_type_ids,
                    ) {
                        Ok(execution_node) => {
                            let execution_node = Arc::new(Mutex::new(execution_node));
                            initialized_nodes.insert(node_id, execution_node);
                            tracing::debug!("[Executor] Node {} initialized.", node_id);
                        }

                        Err(e) => {
                            tracing::error!(
                                "[ERROR] Failed to initialize node {}: {}",
                                node_id,
                                e.to_string()
                            );
                            return Err(ExecutionError::NodeSetupFailed {
                                message: format!("Node {} setup failed: {}", node_id, e),
                            });
                        }
                    }
                }

                Some(NodeConfig::RemoteNodeConfig(runtime_id)) => {
                    tracing::warn!(
                        "[Executor] Skipping remote node {} (belongs to runtime {})",
                        node_id,
                        runtime_id
                    );
                }

                None => {
                    return Err(ExecutionError::NodeSetupFailed {
                        message: format!("Node {} was not found in ExecutionConfig", node_id),
                    });
                }
            }
        }

        if initialized_nodes.is_empty() {
            return Err(ExecutionError::NodeSetupFailed {
                message: "No local nodes were initialized.".to_string(),
            });
        }

        self.execution_nodes = initialized_nodes;

        tracing::info!(
            "[Executor] Successfully initialized {} local nodes.",
            self.execution_nodes.len()
        );

        Ok(())
    }

    /// **Handles Local Node Creation**
    fn create_local_execution_node(
        &self,
        node: Box<dyn Node>,
        node_id: NodeId,
        execution_mode: ExecutionMode,
        input_type_ids: HashMap<NodeIOIndex, TypeId>,
    ) -> Result<ExecutionNode, ExecutionError> {
        let thread_comm =
            ThreadCommunicator::<String>::new().map_err(|err| ExecutionError::NodeSetupFailed {
                message: format!(
                    "Failed to create thread communicator for node {}: {}",
                    node_id, err
                ),
            })?;

        let node_comm = NodeCommunicator::ThreadComm(thread_comm);
        let control_edge = Edge::<String>::new(node_comm);

        Ok(ExecutionNode::new(
            node,
            node_id,
            execution_mode,
            control_edge,
            input_type_ids,
        ))
    }
    /// **Ensure all nodes are in ready state before execution**
    pub async fn ready_nodes(&self) -> Result<(), anyhow::Error> {
        tracing::debug!("[Executor] Ensuring all nodes are in ready state...");

        for (node_id, node) in &self.execution_nodes {
            let mut node_guard = node.lock().await;
            if let Err(e) = node_guard.on_ready() {
                tracing::error!(
                    "[Executor] ERROR: Node {} failed to enter ready state: {}",
                    node_id,
                    e
                );
                return Err(anyhow::Error::msg("Node ready state failed"));
            }
        }

        tracing::info!("[Executor] All nodes are ready.");
        Ok(())
    }

    /// **Starts execution using a Tokio task per node.**
    pub async fn start_execution(&self) {
        tracing::info!("[Executor] Starting execution...");

        for (node_id, execution_node) in &self.execution_nodes {
            let node_id = *node_id;
            let execution_node: Arc<Mutex<ExecutionNode>> = Arc::clone(execution_node);
            tokio::spawn(async move {
                tracing::debug!("[Executor] Running node {}...", node_id);

                let mut node = execution_node.lock().await;
                if let Err(e) = node.on_update_async().await {
                    tracing::error!("[Executor] Error executing node {}: {:?}", node_id, e);
                }
            });
        }

        tracing::debug!("[Executor] Execution started.");
    }

    pub async fn connect_local(
        &mut self,
        sender_id: NodeId,
        receiver_id: NodeId,
        sender_out_idx: u128,
        recv_in_idx: u128,
    ) -> Result<()> {
        // 1) Locate sender ExecutionNode
        let sender_exec_node = self
            .execution_nodes
            .get(&sender_id)
            .ok_or_else(|| anyhow::anyhow!("No ExecutionNode found for sender_id {}", sender_id))?;
        // 2) Locate receiver ExecutionNode
        let receiver_exec_node = self.execution_nodes.get(&receiver_id).ok_or_else(|| {
            anyhow::anyhow!("No ExecutionNode found for receiver_id {}", receiver_id)
        })?;

        // Lock both
        let mut sender_guard = sender_exec_node.lock().await;
        let mut receiver_guard = receiver_exec_node.lock().await;

        // 3) Tell the typed node to create local output communicator
        sender_guard.setup_output(sender_out_idx, /*local=*/ true);

        // 4) Tell the typed node to create local input communicator
        receiver_guard.setup_input(recv_in_idx, /*local=*/ true);

        Ok(())
    }
}
