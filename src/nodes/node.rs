//use anyhow::Result;
use anyhow::anyhow;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    str::FromStr,
    sync::mpsc::{channel, Receiver, Sender},
};
use thiserror::Error;
use tokio::time::{sleep, Duration};

use crate::{
    comm::messages::Message,
    exec::{execution_mode::ExecutionMode, execution_state::ExecutionState},
    flow::flow_types::NodeIOIndex,
};

use super::{
    connection::Edge,
    node_io::{get_input_edge_mut, SetupIO},
};

/// A node can take a shared reference to a [`Context`] instance.
/// There exists a single context for all nodes that can be accessed via mutex.
/// It can be used for sharing instances between nodes (e.g. a wgpu context.)
pub struct Context {
    /// A generic key-value store for instance sharing across nodes.
    pub properties: HashMap<String, Box<dyn Any>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}

/// Node outputs take an object of this type in order to notify an obeserver
/// (usually a flow executor implementing the [`Executor`](crate::exec::execution::Executor) trait)
/// if something happened (which means something was written to an output).
pub struct ChangeObserver {
    /// The notifier as a sender
    /// (usually the [`Output`](crate::nodes::connection::Output) implementation).
    pub notifier: Sender<bool>,

    /// The observer as a receiver
    /// (usually a flow executor implementing the [`Executor`](crate::exec::execution::Executor) trait).
    pub observer: Receiver<bool>,
}

impl ChangeObserver {
    pub fn new() -> Self {
        let (sender, receiver) = channel();

        Self {
            notifier: sender,
            observer: receiver,
        }
    }

    pub fn wait_for_changes(&self) {
        // Wait for a change message.
        // If first message received, get all others.
        let _ = self.observer.recv();
        loop {
            match self.observer.try_recv() {
                Ok(_) => (),
                Err(_) => break,
            }
        }
    }
}

/// Trait that defines the interface of update controller mechanisms.
/// Update controllers are used to cancel long-running [`Node::on_update`] methods.
pub trait UpdateController {
    /// This method is called "from outside" (potentially also different execution thread).
    /// It should implement the logic to cancel the long-running [`Node::on_update`] execution.
    fn cancel(&mut self);
}

/// Trait that has to be implemented by any node.
/// Contains methods for each state in the lifecycle of a node.
pub trait Node: Send + Sync {
    /// This method changes the current execution mode of the node
    fn set_execution_mode(&mut self, _mode: ExecutionMode) -> ExecutionMode {
        ExecutionMode::Continuous
    }

    /// This method retrieves the current execution mode of the node
    fn get_execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Continuous
    }

    /// This method is called for node initialization.
    fn on_init(&mut self) -> Result<(), InitError> {
        Ok(())
    }

    /// This method is called when all nodes in the flow are initialized.
    fn on_ready(&mut self) -> Result<(), ReadyError> {
        Ok(())
    }

    /// This method is called when flow execution ends.
    fn on_shutdown(&mut self) -> Result<(), ShutdownError> {
        Ok(())
    }

    /// This method is called by the executor dependent on its update strategy.
    fn on_update(&mut self) -> Result<(), UpdateError> {
        Ok(())
    }

    // /// Some nodes might have a long-running task in their [`Node::on_update`] method.
    // /// In this case, this method can return an [`UpdateController`] instance which can
    // /// be used for cancelling the update.
    // fn update_controller(&self) -> Option<Box<dyn UpdateController>> {
    //     None
    // }

    fn get_input_count(&self) -> u128;
    fn get_output_count(&self) -> u128;
    fn setup_input(&mut self, idx: u128, local: bool);
    fn setup_output(&mut self, idx: u128, local: bool);
    fn get_io_mut(&mut self) -> &mut dyn SetupIO;
}
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

    // fn get_input_count(&self) -> u128 {
    //     self.node.get_input_count() // assume this is implemented per node
    // }

    // fn get_input_edge(&mut self, idx: usize) -> Option<&mut dyn Any> {
    //     self.node.get_input(idx);
    // }

    // async fn probe_inputs(&mut self) {
    //     for idx in 0..self.get_input_count() {
    //         if let Some(any_edge) = self.get_input_edge(idx) {
    //             if let Some(edge) = any_edge.downcast_mut::<Edge<_>>() {
    //                 match edge.try_message().await {
    //                     Ok(Some(Message::Data(data))) => edge.set_buffer(Some(data.get_data())),
    //                     Ok(Some(_ctrl)) => {} // Control message — ignore for now
    //                     Ok(None) => edge.set_buffer(None), // explicitly set None
    //                     Err(e) => {
    //                         println!("[WARN] Failed to receive message on input {idx}: {}", e);
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // async fn poll_and_buffer_all(&mut self) -> Result<(), UpdateError> {
    //     let io = self.node.get_io_mut();
    //     let input_count = io.get_input_count();

    //     for idx in 0..input_count {
    //         // NOTE: get_input_edge_mut is already defined in node_io.rs
    //         if let Some(edge_any) = get_input_edge_mut(io, idx) {
    //             if let Err(e) = edge_any.poll_and_buffer().await {
    //                 match e {
    //                     ReceiveError::ControlMessage(msg) => {
    //                         return Err(UpdateError::ControlMessage(msg));
    //                     }
    //                     ReceiveError::Other(err) => {
    //                         return Err(UpdateError::RecvError {
    //                             message: err.to_string(),
    //                         });
    //                     }
    //                     ReceiveError::NoMessageAvailable => {
    //                         // This is expected in a non-blocking poll; just skip
    //                     }
    //                 }
    //             }
    //         }
    //     }

    //     Ok(())
    // }

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

                    // Control message check
                    match self.control_edge.try_message().await {
                        Ok(Some(msg)) => {
                            println!("[ExecutionNode] Control message received: {:?}", msg);
                            self.on_message(msg);
                        }
                        Ok(None) => {
                            println!("[ExecutionNode] No control message available.");
                        }
                        Err(e) => match e {
                            ReceiveError::ControlMessage(msg) => {
                                println!("[ExecutionNode] Control message error: {:?}", msg);
                                return Err(UpdateError::ControlMessage(msg));
                            }
                            ReceiveError::Other(e) => {
                                println!("[ExecutionNode] Receive error: {}", e);
                                return Err(UpdateError::RecvError {
                                    message: e.to_string(),
                                });
                            }
                            ReceiveError::NoMessageAvailable => {
                                println!("[ExecutionNode] No control message available.");
                            }
                        },
                    }

                    // Poll all inputs using the SetupIO trait
                    println!("[ExecutionNode] Polling all inputs...");
                    let io = self.node.get_io_mut();
                    match io.poll_inputs().await {
                        Ok(_) => {
                            println!("[ExecutionNode] Input polling completed.");
                        }
                        Err(ReceiveError::ControlMessage(msg)) => {
                            println!(
                                "[ExecutionNode] ⚠ Control message during input polling: {:?}",
                                msg
                            );
                            return Err(UpdateError::ControlMessage(msg));
                        }
                        Err(e) => {
                            println!("[ExecutionNode] Error polling inputs: {:?}", e);
                            return Err(UpdateError::RecvError {
                                message: e.to_string(),
                            });
                        }
                    }

                    // Call on_update
                    println!("[ExecutionNode] ⚙ Calling node.on_update()...");
                    result = self.node.on_update();

                    match result {
                        Ok(_) => println!("[ExecutionNode] Node logic executed successfully."),
                        Err(ref e) => println!("[ExecutionNode] Node logic error: {:?}", e),
                    }

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

    // fn update_controller(&self) -> Option<Box<dyn UpdateController>> {
    //     None
    // }

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

#[derive(Error, Debug)]
pub enum InitError {
    //TODO: Add init specific errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug, Error)]
pub enum ReadyError {
    // General ready-specific errors
    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Allow conversion from &str to ReadyError
impl From<&str> for ReadyError {
    fn from(msg: &str) -> Self {
        ReadyError::Message(msg.to_string())
    }
}

#[derive(Error, Debug)]
pub enum ShutdownError {
    //TODO: Add shutdown specific errors.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Debug)]
pub struct SequenceError {
    pub node: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[derive(Error, Debug)]
pub enum ReceiveError<D>
where
    D: Clone + fmt::Debug + FromStr,
    D: Send + 'static,
{
    #[error(transparent)]
    Other(#[from] anyhow::Error),
    ControlMessage(Message<D>),
    NoMessageAvailable,
}

// impl<D> fmt::Display for ReceiveError<D>
// where
//     D: Clone + fmt::Debug + FromStr,
// {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self)
//     }
// }

impl<D> ToString for ReceiveError<D>
where
    D: Clone + fmt::Debug + FromStr,
    D: Send + 'static,
{
    fn to_string(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Error, Debug)]
pub enum UpdateError {
    #[error("Sequence error. Message: {message:?}")]
    SequenceError { message: String },

    #[error("Connect error. Message: {message:?}")]
    ConnectError { message: String },

    #[error("SendError error. Message: {message:?}")]
    SendError { message: String },

    #[error("RecvError error. Message: {message:?}")]
    RecvError { message: String },

    #[error("NotReadyError error. Message: {message:?}")]
    NotReadyError { message: String },

    #[error("AlreadyRunningError error. Message: {message:?}")]
    AlreadyRunningError { message: String },

    #[error("Received control message on data edge: {0:?}")]
    ControlMessage(Message<String>),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<SendError> for UpdateError {
    fn from(value: SendError) -> Self {
        UpdateError::SendError {
            message: value.to_string(),
        }
    }
}

impl<D> From<ReceiveError<D>> for UpdateError
where
    D: Clone + fmt::Debug + FromStr,
    D: Send + 'static,
{
    fn from(value: ReceiveError<D>) -> Self {
        UpdateError::SendError {
            message: value.to_string(),
        }
    }
}

pub async fn poll_all_inputs(io: &mut dyn SetupIO) -> Result<(), ReceiveError<String>> {
    let count = io.get_input_count();
    for idx in 0..count {
        if let Some(edge_any) = get_input_edge_mut::<String>(io, idx) {
            edge_any.poll_and_buffer().await?;
        }
    }
    Ok(())
}
