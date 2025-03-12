// use std::collections::hash_map::Drain;
// use std::collections::HashMap;
// use std::net::SocketAddr;
// use std::{env, thread, time::Duration};

// use anyhow::Result;
// use metrics::increment_counter;
// #[cfg(feature = "metrics")]
// use metrics_exporter_prometheus::PrometheusBuilder;
// use thiserror::Error;
// use tracing::metadata::LevelFilter;
// use tracing::{error, info_span};

// #[cfg(feature = "tracing")]
// use crate::analytics::otlp_exporter::OtlpExporter;
// use crate::comm::communication::{Communicator, NodeCommunicator};
// use crate::comm::network_communicator::NetworkCommunicator;
// use crate::comm::thread_communicator::ThreadCommunicator;
// use crate::connection::Edge;
// use crate::flow::execution_flow::ExecutionFlow;
// use crate::flow::flow_types::NodeId;
// use crate::node::{ExecutionNode, Node};
// use crate::{
//     exec::node_updater::{NodeUpdateError, NodeUpdater, SleepMode},
//     flow::abstract_flow::AbstractFlow,
//     scheduler::{Scheduler, SchedulingInfo},
// };

// use super::execution_configuration::{ExecutionConfig, ExecutionConfigError, NodeConfig};
// use super::execution_mode::ExecutionMode;

// cfg_if::cfg_if! {
//     if #[cfg(feature = "tracing")] {
//         use opentelemetry::trace::TracerProvider as _;
//         use opentelemetry_sdk::trace::TracerProvider;
//         use opentelemetry_sdk::Resource;
//         use tracing_subscriber::layer::SubscriberExt;
//         use tracing_subscriber::Registry;
//     }
// }
// pub struct ExecutionContext {
//     pub executor: StandardExecutor,
//     pub flow: AbstractFlow,
// }

// impl ExecutionContext {
//     pub fn new(executor: StandardExecutor, flow: AbstractFlow) -> Self {
//         Self {
//             executor: executor,
//             flow: flow,
//         }
//     }
// }

// #[repr(C)]
// #[allow(dead_code)]
// pub struct ExecutionContextHandle {
//     _data: [u8; 0],
//     _marker: core::marker::PhantomData<(*mut u8, core::marker::PhantomPinned)>,
// }

// pub trait Executor {
//     fn run<S, U>(&mut self, flow: AbstractFlow, scheduler: S, node_updater: U) -> Result<()>
//     where
//         S: Scheduler + std::marker::Send,
//         U: NodeUpdater + Drop;

//     async fn setup_and_connect(
//         &mut self,
//         abstract_flow: AbstractFlow,
//         execution_config: ExecutionConfig,
//     ) -> Result<ExecutionFlow, ExecutionError>;
// }

// #[derive(Error, Debug)]
// pub enum ExecutionError {
//     #[error("Errors occured while updating nodes: {errors:?}")]
//     UpdateErrorCollection { errors: Vec<NodeUpdateError> },

//     #[error("Node Setup Failed Error. Message: {message:?}")]
//     NodeSetupFailed { message: String },
// }

// pub struct StandardExecutor {
//     // contains the known network communicators for the executor
//     //known_communicators: HashMap<SocketAddr, NodeCommunicator<String>>,
// }

// impl StandardExecutor {
//     pub fn new() -> Self {
//         Self {
//             //known_communicators: HashMap::new(),
//         }
//     }

//     #[tracing::instrument(skip_all)]
//     fn run_update_loop<S, U>(
//         &mut self,
//         flow: &mut AbstractFlow,
//         mut scheduler: S,
//         mut node_updater: U,
//     ) -> Result<(), ExecutionError>
//     where
//         S: Scheduler,
//         U: NodeUpdater,
//     {
//         //self.controller.set_state(ExecutionState::Running);

//         let mut info = SchedulingInfo::new(flow.num_nodes());

//         //let mut update_controllers = flow.get_update_controllers();

//         //while !self.controller.cancellation_requested() {
//         increment_counter!("flowrs.executions");
//         // Run an epoch (an update of each node).
//         scheduler.restart_epoch(&mut info);

//         //println!("                                                                                                    {:?} NEW EPOCH", std::thread::current().id());
//         while !scheduler.epoch_is_over(&mut info) {
//             let node_idx = scheduler.get_next_node_idx();
//             //println!("                                                                                                    {:?} {}", std::thread::current().id(), node_idx);

//             // let (node_description, node_id);
//             // {
//             //     // Borrow `flow` immutably to get the description.
//             //     if let Some(n) = flow.node_by_index(node_idx) {
//             //         node_id = n.0;
//             //         node_description = flow.node_description_by_id(node_id).cloned();
//             //     } else {
//             //         continue;
//             //     }
//             // }

//             // Now, borrow `flow` mutably to update the node.

//             // if let Some(n) = flow.node_by_index(node_idx) {
//             //     if let Some(description) = node_description {
//             //         node_updater.update((n.0, &mut n.1), Some(description));
//             //     } else {
//             //         node_updater.update((n.0, &mut n.1), None);
//             //     }
//             // }

//             // let node = flow.node_by_index(node_idx);

//             // if let Some(n) = node {
//             //     let description = flow.node_description_by_id(n.0);
//             //     let mut n1 = &mut *n.1;
//             //     node_updater.update((n.0, &n.1), description.cloned());
//             // }
//         }

//         // Sleep if necessary.
//         {
//             let _sleep_span = info_span!("sleep").entered();
//             match node_updater.sleep_mode() {
//                 SleepMode::None => {}

//                 SleepMode::Reactive => {
//                     //self.controller.set_state(ExecutionState::Sleeping);

//                     //self.observer.wait_for_changes();

//                     //self.controller.set_state(ExecutionState::Running);
//                 }

//                 SleepMode::FixedFrequency(fps) => {
//                     let actual_duration = info.epoch_duration;
//                     let target_duration = Duration::from_millis(1000 / fps);
//                     let delta = target_duration.saturating_sub(actual_duration);
//                     //println!("AD: {:?} TD: {:?} DELTA: {:?}", actual_duration, target_duration, delta);
//                     if delta > Duration::ZERO {
//                         thread::sleep(delta);
//                     }
//                 }
//             }
//         }

//         // Check if async errors occured.
//         // let errors: Vec<NodeUpdateError> = node_updater
//         //     .errors()
//         //     .into_iter()
//         //     .map(|mut err| {
//         //         if let Some(id) = err.node_id {
//         //             err.node_id = Some(id);
//         //             if let Some(desc) = flow.node_description_by_id(id) {
//         //                 err.node_desc = Some(desc.clone());
//         //             }
//         //         }
//         //         err
//         //     })
//         //     .collect();
//         // if !errors.is_empty() {
//         //     return Err(ExecutionError::UpdateErrorCollection { errors });
//         // }
//         //}

//         // Cancel long-running node updates.
//         //update_controllers.iter_mut().for_each(|uc| uc.cancel());

//         // Drop node updater which destroys all workers.
//         //drop(node_updater);

//         // All done.
//         //self.controller.set_state(ExecutionState::Ready);

//         Ok(())
//     }

//     /// Private helper function
//     /// handles network node configurations
//     #[inline]
//     async fn handle_network_node_config(
//         &mut self,
//         ip: &SocketAddr,
//         node: Box<dyn Node>,
//         execution_mode: ExecutionMode,
//     ) -> Result<ExecutionNode, ExecutionConfigError> {
//         // if let Some(node_comm) = self.known_communicators.remove(ip) {
//         //     //let node_comm = NodeCommunicator::<String>::NetworkComm(net_comm);
//         //     let control_edge = Edge::<String>::new(node_comm);
//         //     let exec_node = ExecutionNode::new(node, execution_mode, control_edge);
//         //     self.known_communicators.insert(*ip, node_comm);
//         //     Ok(exec_node)
//         // } else {
//         let mut net_comm = NetworkCommunicator::new().await.unwrap();
//         let connect_res = <NetworkCommunicator as Communicator<String>>::connect_send(
//             &mut net_comm,
//             Some(ip.ip().to_string()),
//             Some(ip.port()),
//         )
//         .await;
//         match connect_res {
//             Ok(_) => {
//                 let node_comm = NodeCommunicator::<String>::NetworkComm(net_comm);
//                 let control_edge = Edge::<String>::new(node_comm);
//                 let exec_node = ExecutionNode::new(node, execution_mode, control_edge);
//                 Ok(exec_node)
//             }
//             Err(err) => Err(ExecutionConfigError::CommunicationSetupFailed {
//                 message: format!(
//                     "Connecting as Sender to Network Runtime [{}:{}has failed. {:?}",
//                     ip.ip(),
//                     ip.port(),
//                     err
//                 ),
//             }),
//         }
//         //}
//     }

//     /// Private helper function
//     /// handles local node configurations
//     /// creates a local (Thread) communicator
//     /// returns initialized execution node
//     #[inline]
//     async fn handle_local_node_config(
//         &self,
//         node_id: NodeId,
//         node: Box<dyn Node>,
//         execution_mode: ExecutionMode,
//     ) -> Result<ExecutionNode, ExecutionConfigError> {
//         match ThreadCommunicator::<String>::new() {
//             Ok(thread_comm) => {
//                 let node_comm = NodeCommunicator::ThreadComm(thread_comm);
//                 let control_edge = Edge::<String>::new(node_comm);
//                 let exec_node = ExecutionNode::new(node, execution_mode, control_edge);
//                 Ok(exec_node)
//             }
//             Err(err) => Err(ExecutionConfigError::CommunicationSetupFailed {
//                 message: format!(
//                     "Failed to create thread communicator for node {}: {}",
//                     node_id, err
//                 ),
//             }),
//         }
//     }

//     /// Private helper function
//     /// handles missing execution configurations
//     #[inline]
//     fn handle_missing_execution_config(node_id: NodeId) -> ExecutionConfigError {
//         ExecutionConfigError::MissingExecutionConfig {
//             message: format!("No ExecutionConfig found for node {}", node_id),
//         }
//     }

//     /// Private helper function
//     /// handles execution node creation from nodes according to their config
//     #[inline]
//     async fn create_execution_nodes<'a>(
//         &mut self,
//         nodes: Drain<'a, u128, Box<dyn Node>>,
//         execution_config: &ExecutionConfig,
//         execution_mode: ExecutionMode,
//     ) -> Result<HashMap<u128, Box<dyn Node>>, ExecutionError> {
//         //     let mut results = Vec::new();

//         //     for (node_id, node) in nodes {
//         //         let execution_mode = execution_mode.clone();
//         //         let node_config = execution_config.node_configs.get(&node_id);

//         //         let result = match node_config {
//         //             Some(NodeConfig::RemoteNodeConfig(ip)) => {
//         //                 self.handle_network_node_config(ip, node, execution_mode)
//         //                     .await
//         //             }
//         //             Some(NodeConfig::LocalNodeConfig) => {
//         //                 self.handle_local_node_config(node_id, node, execution_mode)
//         //                     .await
//         //             }
//         //             None => Err(StandardExecutor::handle_missing_execution_config(node_id)),
//         //         };

//         //         // store the result with its node_id
//         //         results.push((node_id, result));
//         //     }

//         //     // convert results into a HashMap
//         //     let node_map = results
//         //         .into_iter()
//         //         .map(|(node_id, node_res)| match node_res {
//         //             Ok(exec_node) => Ok((node_id, Box::new(exec_node) as Box<dyn Node>)),
//         //             Err(err) => {
//         //                 eprintln!("Error setting up node {}: {:?}", node_id, err);
//         //                 Err(ExecutionError::NodeSetupFailed {
//         //                     message: format!("Error setting up node {}: {:?}", node_id, err,),
//         //                 })
//         //             }
//         //         })
//         //         .collect::<Result<HashMap<_, _>, _>>()?;
//         //     Ok(node_map)

//         Ok(HashMap::new())
//     }
// }

// impl Executor for StandardExecutor {
//     fn run<S, U>(
//         &mut self,
//         mut abstract_flow: AbstractFlow,
//         scheduler: S,
//         node_updater: U,
//     ) -> Result<(), anyhow::Error>
//     where
//         S: Scheduler + std::marker::Send,
//         U: NodeUpdater + Drop,
//     {
//         let mut runner = || {
//             // // Trace executed code
//             // // Spans will be sent to the configured OpenTelemetry exporter
//             // let _root = info_span!("executor_run").entered();

//             // //TODO STEP ONE: Read in Environment Config
//             // //      --> Not yet implemented, skip
//             // let mut execution_config = ExecutionConfig::new();
//             // execution_config.node_configs = abstract_flow
//             //     .get_nodes()
//             //     .map(|(node_id, node)| (*node_id, NodeConfig::LocalNodeConfig))
//             //     .collect();

//             // //TODO STEP TWO: Read in Flow (abstract but typed representation)
//             // //not needed -> given as parameter

//             // //Step 3: Create ExecutionFlow (flow structure which is no longer abstract)
//             // let mut execution_flow: ExecutionFlow = ExecutionFlow::new_empty();
//             // let execution_mode = ExecutionMode::Continuous;

//             // let connections = abstract_flow.move_connections();
//             // execution_flow.set_connections(connections);
//             // let nodes = abstract_flow.move_nodes();
//             // // 3.2: Add all ExecutionNodes to the ExecutionFlow Structure
//             // //create ExecutionNodes for each Node using their respective NodeConfigs to determine where to run

//             // //store known network communicators to controll remote runners in a map
//             // let known_communicators: HashMap<SocketAddr, NetworkCommunicator> = HashMap::new();

//             //Step 4: Setup Phase. Initialize all Nodes on their runners, then connect them together correctly
//             //4.1: Initialize all nodes on their runners
//             //4.2: Connect Nodes

//             //TODO STEP FOUR: Connect ExecutionFlow (SETUP PHASE)
//             // let connections = abstract_flow.get_connections();
//             // connections.for_each(|connection| {
//             //     connection.
//             // });

//             //TODO STEP FIVE: Start Execution using pre-determined scheduling

//             //TODO STEP SIX: After Execution, Tear Down Nodes and return Result

//             // abstract_flow
//             //     .init_all()
//             //     .context(format!("Unable to init all nodes."))?;

//             // abstract_flow
//             //     .ready_all()
//             //     .context(format!("Unable to make all nodes ready."))?;

//             // self.run_update_loop(&mut abstract_flow, scheduler, node_updater)?;

//             // abstract_flow
//             //     .shutdown_all()
//             //     .context(format!("Unable to shutdown all nodes"))?;

//             #[cfg(feature = "metrics")]
//             {
//                 let pid = std::process::id().to_string();
//                 let client = reqwest::blocking::Client::new();
//                 let pushgateway_host =
//                     env::var("PUSHGATEWAY_HOST").unwrap_or("http://localhost:9091".to_string());
//                 let url = format!("{pushgateway_host}/metrics/job/flowrs-{pid}");
//                 let _ = client.delete(&url).send();
//             }

//             Ok(())
//         };

//         #[cfg(feature = "metrics")]
//         {
//             let pid = std::process::id().to_string();

//             let pushgateway_host =
//                 env::var("PUSHGATEWAY_HOST").unwrap_or("http://localhost:9091".to_string());

//             PrometheusBuilder::new()
//                 .add_global_label("pid", &pid)
//                 .with_push_gateway(
//                     format!("{pushgateway_host}/metrics/job/flowrs-{pid}"),
//                     Duration::from_secs(1),
//                     None,
//                     None,
//                 )
//                 .expect("Invalid push gateway configuration")
//                 .install()
//                 .expect("failed to install recorder/exporter");
//         }

//         #[cfg(feature = "tracing")]
//         {
//             // Create a resource configuration
//             let resource =
//                 Resource::new(vec![opentelemetry::KeyValue::new("service.name", "flowrs")]);

//             let tempo_host =
//                 env::var("TEMPO_HOST").unwrap_or("http://localhost:4318/v1/traces".to_string());

//             // Create a new OpenTelemetry trace pipeline that prints to stdout
//             let provider = TracerProvider::builder()
//                 .with_config(opentelemetry_sdk::trace::Config::default().with_resource(resource))
//                 .with_simple_exporter(OtlpExporter::new(tempo_host))
//                 .build();
//             let tracer = provider.tracer("flowrs");

//             // Create a tracing layer with the configured tracer
//             let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);

//             // Use the tracing subscriber `Registry`, or any other subscriber
//             // that impls `LookupSpan`
//             let subscriber = Registry::default().with(telemetry).with(LevelFilter::INFO);

//             tracing::subscriber::set_global_default(subscriber)
//                 .expect("Failed to set the global default tracing subscriber");

//             // Setup LogTracer and env_logger at the same time
//             let combined_logger = crate::analytics::combined_logger::CombinedLogger::new();

//             // combine both loggers together
//             log::set_boxed_logger(Box::new(combined_logger))
//                 .map(|()| log::set_max_level(log::LevelFilter::Info))
//                 .expect("Failed to set logger");

//             log::info!("Starting flowrs");
//             return runner();
//         }

//         #[cfg(not(feature = "tracing"))]
//         {
//             env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"))
//                 .target(env_logger::Target::Stdout)
//                 .init();

//             return runner();
//         }
//     }

//     async fn setup_and_connect(
//         &mut self,
//         mut abstract_flow: AbstractFlow,
//         execution_config: ExecutionConfig,
//     ) -> Result<ExecutionFlow, ExecutionError> {
//         // create ExecutionMode
//         // StandardExecutor operates continuously
//         let execution_mode = ExecutionMode::Continuous;

//         // create execution flow
//         let mut execution_flow = ExecutionFlow::new_empty();

//         // move connections to execution flow
//         let connections = abstract_flow.move_connections();
//         execution_flow.set_connections(connections);

//         // create executionNodes and move to execution flow
//         let nodes = abstract_flow.move_nodes();
//         let node_map = self
//             .create_execution_nodes(nodes, &execution_config, execution_mode)
//             .await?;
//         execution_flow.set_nodes(node_map);

//         // connect with control edges of each node

//         // connect nodes into a flow

//         Ok(execution_flow)
//     }

//     // fn controller(&self) -> ExecutionController {
//     //     self.controller.clone()
//     // }
// }
