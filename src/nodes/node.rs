use std::{any::Any, collections::HashMap, sync::{mpsc::{Sender, Receiver, channel}, Arc, Mutex}};
use thiserror::Error;
use anyhow::Result;

/// A node can take a shared reference to a [`Context`] instance. 
/// There exists a single context for all nodes that can be accessed via mutex. 
/// It can be used for sharing instances between nodes (e.g. a wgpu context.)
pub struct Context {
    /// A generic key-value store for instance sharing across nodes.
    pub properties: HashMap<String, Box<dyn Any>>
}

impl Context {
    pub fn new() -> Self {
        Self {
            properties: HashMap::new()
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

    pub fn wait_for_changes(&self){
        
        // Wait for a change message.
        // If first message received, get all others.  
        let _  = self.observer.recv();
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
pub trait Node : Send {
    /// This method is called for node initialization.
    fn on_init(&self) -> Result<(), InitError> { Ok(())}

    /// This method is called when all nodes in the flow are initialized.
    fn on_ready(&self) -> Result<(), ReadyError> { Ok(())}

    /// This method is called when flow execution ends.
    fn on_shutdown(&self) -> Result<(), ShutdownError> { Ok(())}

    /// This method is called by the executor dependent on its update strategy. 
    fn on_update(&mut self) -> Result<(), UpdateError> { Ok(())}

    /// Some nodes might have a long-running task in their [`Node::on_update`] method. 
    /// In this case, this method can return an [`UpdateController`] instance which can 
    /// be used for cancelling the update.
    fn update_controller(&self) -> Option<Arc<Mutex<dyn UpdateController>>> { None}
}

#[derive(Error, Debug)]
pub enum InitError {
    
    //TODO: Add init specific errors.

    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 

#[derive(Error, Debug)]
pub enum ReadyError {
    
    //TODO: Add ready specific errors.

    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 

#[derive(Error, Debug)]
pub enum ShutdownError {
    
    //TODO: Add shutdown specific errors.

    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 

#[derive(Debug)]
pub struct SequenceError {
    pub node: String,
    pub message: String,
}

#[derive(Error, Debug)]
pub enum SendError {
    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

#[derive(Error, Debug)]
pub enum ReceiveError {
    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

#[derive(Error, Debug)]
pub enum UpdateError {

    #[error("Sequence error. Message: {message:?}")]
    SequenceError {
        message: String,
    },

    #[error("Connect error. Message: {message:?}")]
    ConnectError {
        message: String,
    },

    #[error("SendError error. Message: {message:?}")]
    SendError {
        message: String,
    },

    #[error("RecvError error. Message: {message:?}")]
    RecvError {
        message: String,
    },


    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

impl From<SendError> for UpdateError {
    fn from(value: SendError) -> Self {
        UpdateError::SendError { message: value.to_string() }
    }
}

impl From<ReceiveError> for UpdateError {
    fn from(value: ReceiveError) -> Self {
        UpdateError::SendError { message: value.to_string() }
    }
}
