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
        abstract_flow: &mut AbstractFlow,
        execution_config: &ExecutionConfig,
    ) -> Result<(), ExecutionError> {
        println!("[Executor] Initializing local nodes...");

        let mut initialized_nodes = HashMap::new();

        for (node_id, node) in abstract_flow.move_nodes() {
            match execution_config.node_configs.get(&node_id) {
                Some(NodeConfig::LocalNodeConfig) => {
                    let execution_node = Arc::new(Mutex::new(
                        self.create_local_execution_node(
                            node,
                            node_id,
                            self.execution_mode.clone(),
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
    ) -> Result<ExecutionNode, ExecutionError> {
        match ThreadCommunicator::<String>::new() {
            Ok(thread_comm) => {
                let node_comm = NodeCommunicator::ThreadComm(thread_comm);
                let control_edge = Edge::<String>::new(node_comm);
                Ok(ExecutionNode::new(node, execution_mode, control_edge))
            }
            Err(err) => Err(ExecutionError::NodeSetupFailed {
                message: format!(
                    "Failed to create thread communicator for node {}: {}",
                    node_id, err
                ),
            }),
        }
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
    pub async fn start_execution(self: Arc<Self>) {
        println!("[Executor] Starting execution...");

        for (node_id, execution_node) in &self.execution_nodes {
            let node_id = *node_id;
            let execution_node = Arc::clone(execution_node);
            tokio::spawn(async move {
                println!("[Executor] Running node {}...", node_id);

                let mut node = execution_node.lock().await;
                if let Err(e) = node.on_update() {
                    println!("[Executor] Error executing node {}: {:?}", node_id, e);
                }
            });
        }

        println!("[Executor] Execution started.");
    }
}
