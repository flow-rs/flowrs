use std::any::TypeId;
use std::collections::hash_map::Drain;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{env, thread, time::Duration};

use anyhow::Result;
use metrics::increment_counter;
#[cfg(feature = "metrics")]
use metrics_exporter_prometheus::PrometheusBuilder;
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::task;
use tracing::metadata::LevelFilter;
use tracing::{error, info_span};

#[cfg(feature = "tracing")]
use crate::analytics::otlp_exporter::OtlpExporter;
use crate::comm::communication::{Communicator, NodeCommunicator};
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use crate::connection::Edge;
use crate::flow::execution_flow::ExecutionFlow;
use crate::flow::flow_types::NodeId;
use crate::node::{ExecutionNode, Node};
use crate::nodes::connection::EdgeTrait;
use crate::types::type_registry::TYPE_REGISTRY;
use crate::{
    exec::node_updater::{NodeUpdateError, NodeUpdater, SleepMode},
    flow::abstract_flow::AbstractFlow,
    scheduler::{Scheduler, SchedulingInfo},
};

use super::execution_configuration::{ExecutionConfig, ExecutionConfigError, NodeConfig};
use super::execution_mode::ExecutionMode;

cfg_if::cfg_if! {
    if #[cfg(feature = "tracing")] {
        use opentelemetry::trace::TracerProvider as _;
        use opentelemetry_sdk::trace::TracerProvider;
        use opentelemetry_sdk::Resource;
        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::Registry;
    }
}
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

#[repr(C)]
#[allow(dead_code)]
pub struct ExecutionContextHandle {
    _data: [u8; 0],
    _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
}

pub trait Executor {
    fn run<S, U>(&mut self, flow: AbstractFlow, scheduler: S, node_updater: U) -> Result<()>
    where
        S: Scheduler + std::marker::Send,
        U: NodeUpdater + Drop;

    // async fn setup_and_connect(
    //     &mut self,
    //     abstract_flow: AbstractFlow,
    //     execution_config: ExecutionConfig,
    // ) -> Result<ExecutionFlow, ExecutionError>;
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Errors occured while updating nodes: {errors:?}")]
    UpdateErrorCollection { errors: Vec<NodeUpdateError> },

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
        println!("[Executor] Initializing local nodes...");

        let mut initialized_nodes = HashMap::new();

        for (node_id, node) in abstract_flow.lock().await.move_nodes() {
            //let abstract_flow_guard = abstract_flow.lock().await;

            match execution_config.node_configs.get(&node_id) {
                Some(NodeConfig::LocalNodeConfig) => {
                    let execution_node = Arc::new(Mutex::new(
                        self.create_local_execution_node(
                            node,
                            node_id,
                            self.execution_mode.clone(),
                            Arc::clone(&abstract_flow),
                        )
                        .await?,
                    ));
                    initialized_nodes.insert(node_id, execution_node);
                    println!("[Executor] Node {} initialized.", node_id);
                }

                Some(NodeConfig::RemoteNodeConfig(runtime_id)) => {
                    println!(
                        "[Executor] Skipping remote node {} (belongs to runtime {})",
                        node_id, runtime_id
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
        println!(
            "[Executor] Successfully initialized {} local nodes.",
            self.execution_nodes.len()
        );

        Ok(())
    }

    /// **Handles Local Node Creation**
    async fn create_local_execution_node(
        &self,
        node: Box<dyn Node>,
        node_id: NodeId,
        execution_mode: ExecutionMode,
        abstract_flow: Arc<Mutex<AbstractFlow>>,
    ) -> Result<ExecutionNode, ExecutionError> {
        // Create the control communicator
        let thread_comm =
            ThreadCommunicator::<String>::new().map_err(|err| ExecutionError::NodeSetupFailed {
                message: format!(
                    "Failed to create thread communicator for node {}: {}",
                    node_id, err
                ),
            })?;

        let node_comm = NodeCommunicator::ThreadComm(thread_comm);
        let control_edge = Edge::<String>::new(node_comm);

        // Gather input type IDs for this node from the abstract flow
        let mut input_type_ids = HashMap::new();
        let abstract_flow_guard = abstract_flow.lock().await;
        for conn in abstract_flow_guard.get_connections() {
            if conn.receiver_id == node_id {
                match abstract_flow_guard.get_connection_type(conn) {
                    Some(type_id) => {
                        if let Some((_, type_id)) = abstract_flow_guard.get_connection_type(conn) {
                            input_type_ids.insert(conn.recv_in_idx, type_id);
                        }
                    }
                    None => {
                        return Err(ExecutionError::NodeSetupFailed {
                            message: format!("Missing type ID for input of node {}", node_id),
                        });
                    }
                }
            }
        }
        drop(abstract_flow_guard);

        Ok(ExecutionNode::new(
            node,
            execution_mode,
            control_edge,
            input_type_ids,
        ))
    }

    /// **Ensure all nodes are in ready state before execution**
    pub async fn ready_nodes(&self) -> Result<(), anyhow::Error> {
        println!("[Executor] Ensuring all nodes are in ready state...");

        for (node_id, node) in &self.execution_nodes {
            let mut node_guard = node.lock().await;
            if let Err(e) = node_guard.on_ready() {
                println!(
                    "[Executor] ERROR: Node {} failed to enter ready state: {}",
                    node_id, e
                );
                return Err(anyhow::Error::msg("Node ready state failed"));
            }
        }

        println!("[Executor] All nodes are ready.");
        Ok(())
    }

    /// **Starts execution using a Tokio task per node.**
    pub async fn start_execution(&self) {
        println!("[Executor] Starting execution...");

        for (node_id, execution_node) in &self.execution_nodes {
            let node_id = *node_id;
            let execution_node = Arc::clone(execution_node);
            tokio::spawn(async move {
                println!("[Executor] Running node {}...", node_id);

                let mut node = execution_node.lock().await;
                if let Err(e) = node.on_update_async().await {
                    println!("[Executor] Error executing node {}: {:?}", node_id, e);
                }
            });
        }

        println!("[Executor] Execution started.");
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

    /// Example placeholder that sets the NodeIO's output #out_idx to the given communicator
    fn store_in_output<T>(
        node: &mut dyn Node,
        out_idx: u128,
        comm: NodeCommunicator<T>,
    ) -> anyhow::Result<()>
    where
        T: std::fmt::Debug + std::str::FromStr + Send + Sync + 'static,
    {
        // 1) If your node is e.g. AddNode<I1, I2, O>:
        //    node.io.outputs.0.output = Output::new(comm)
        // or do an internal cast or an interface if the Node trait has a “set_output_comm” method
        // ...
        Ok(())
    }

    /// Same idea for the input
    fn store_in_input<T>(
        node: &mut dyn Node,
        in_idx: u128,
        comm: NodeCommunicator<T>,
    ) -> anyhow::Result<()>
    where
        T: std::fmt::Debug + std::str::FromStr + Send + Sync + 'static,
    {
        //  ...
        Ok(())
    }
}
