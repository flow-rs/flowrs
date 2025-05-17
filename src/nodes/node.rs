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
    exec::{
        execution_directive::NodeExecutionDirective, execution_mode::ExecutionMode,
        execution_state::ExecutionState,
    },
    flow::flow_types::NodeIOIndex,
    types::type_registry::POLL_REGISTRY,
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

/// Trait that has to be implemented by any node
/// Contains methods for each state in the lifecycle of a node
pub trait Node: Send + Sync {
    /// This method is called for node initialization
    fn on_init(&mut self) -> Result<(), InitError> {
        Ok(())
    }

    /// This method is called once all nodes in the flow are initialized
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

    /// This function is called before on_update to control execution behavior
    /// Default directive behavior: wait for all inputs.
    fn on_update_directive(&mut self) -> Result<NodeExecutionDirective, UpdateError> {
        let count = self.get_input_count();
        let required_inputs = (0..count).map(|i| i.into()).collect();

        Ok(NodeExecutionDirective::WaitForInputs(required_inputs))
    }

    /// This method changes the current execution mode of the node
    fn set_execution_mode(&mut self, _mode: ExecutionMode) -> ExecutionMode {
        ExecutionMode::Continuous
    }

    /// This method retrieves the current execution mode of the node
    fn get_execution_mode(&self) -> ExecutionMode {
        ExecutionMode::Continuous
    }

    /// Returns the total number of inputs the node has
    fn get_input_count(&self) -> u128;

    /// Returns the total number of outputs the node has
    fn get_output_count(&self) -> u128;

    /// Sets up an input with index and locality flag
    fn setup_input(&mut self, idx: u128, local: bool);

    /// Sets up an output with index and locality flag
    fn setup_output(&mut self, idx: u128, local: bool);

    /// Retrieves the IO of the node as mutable object
    fn get_io_mut(&mut self) -> &mut dyn SetupIO;
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
