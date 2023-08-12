use std::sync::{mpsc::{Sender, Receiver, channel}, Arc, Mutex};
use thiserror::Error;
use anyhow::Result;

#[derive(Clone)]
pub struct Context {
}

impl Context {
    pub fn new() -> Self {
        Self {
        }
    }
}

pub struct ChangeObserver {
    pub notifier: Sender<bool>,
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

//pub struct UpdateController {
//    cancellation_requested: std::sync::atomic::AtomicBool 
//}

pub trait UpdateController {
    fn cancel(&mut self);
}

pub trait Node : Send {
    fn on_init(&self) -> Result<(), InitError> { Ok(())}
    fn on_ready(&self) -> Result<(), ReadyError> { Ok(())}
    fn on_shutdown(&self) -> Result<(), ShutdownError> { Ok(())}
    fn on_update(&mut self) -> Result<(), UpdateError> { Ok(())}

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

    #[error("RecvError error. Message: {message:?}")]
    RecvError {
        message: String,
    },


    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 
