//use anyhow::Result;
use anyhow::anyhow;
use std::{
    any::Any,
    collections::HashMap,
    fmt,
    str::FromStr,
    sync::mpsc::{channel, Receiver, Sender},
};
use thiserror::Error;
use tokio::time::Duration;

use crate::{
    comm::messages::Message,
    exec::{execution_mode::ExecutionMode, execution_state::ExecutionState},
};

use super::{connection::Edge, node_io::SetupIO};

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
}

impl ExecutionNode {
    pub fn new(
        node: Box<dyn Node + Send + Sync>,
        execution_mode: ExecutionMode,
        control_edge: Edge<String>,
    ) -> Self {
        ExecutionNode {
            node,
            execution_mode: execution_mode,
            execution_state: ExecutionState::Initialized,
            control_edge: control_edge,
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
        println!(
            "[ExecutionNode] Entered on_update_async() with mode: {:?}, state: {:?}",
            self.execution_mode, self.execution_state
        );
        match self.execution_state {
            ExecutionState::Ready => {
                self.execution_state = ExecutionState::Running;
                let mut res = Ok(());

                loop {
                    // Check for shutdown before running node logic
                    if self.execution_state == ExecutionState::Shutdown {
                        if let Err(e) = self.on_shutdown() {
                            println!("[WARN] Shutdown failed: {}", e);
                        }
                        break;
                    }

                    // use `tokio::select!` if control_edge might block
                    tokio::select! {
                        // Control message is available
                        ctrl_msg = self.control_edge.try_message() => {
                            match ctrl_msg {
                                Ok(Some(message)) => self.on_message(message),
                                Ok(None) => {}, // no control message
                                Err(err) => {
                                    return Err(UpdateError::RecvError {
                                        message: err.to_string(),
                                    })
                                }
                            }
                        }

                        // use timeout to ensure on_update() is not starved
                        _ = tokio::time::sleep(Duration::from_millis(10)) => {
                            let execution_res = self.node.on_update();

                            match self.execution_mode {
                                ExecutionMode::Synchronized => {
                                    self.execution_state = ExecutionState::Ready;
                                    res = execution_res;
                                    break;
                                }
                                ExecutionMode::Continuous => {
                                    res = execution_res;
                                    continue;
                                }
                            }
                        }
                    }
                }

                res
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
