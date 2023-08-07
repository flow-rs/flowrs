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
        let _  = self.observer.recv();
    }
}

pub trait Node : Send + 'static {
    fn name(&self) -> &str;

    fn on_init(&self) -> Result<(), InitError>;
    fn on_ready(&self) -> Result<(), ReadyError>;
    fn on_shutdown(&self) -> Result<(), ShutdownError>;
    fn update(&self) -> Result<(), UpdateError>;
}

/*
pub trait SaveableNode {

    save<T> (stream : T)
    load<T>(stream : T ) {

        stream.write(int32)
    }
}
 */

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
pub enum UpdateError {

    #[error("Sequence error for node {node:?}. Message: {message:?}")]
    SequenceError {
        node: String,
        message: String,
    },

    #[error("Sequence error for node {node:?}. Message: {message:?}")]
    ConnectError {
        node: String,
        message: String,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 

pub struct State<S: Clone>(pub Arc<Mutex<S>>);

impl<S: Clone> State<S> {
    pub fn new(inner: S) -> Self {
        State(Arc::new(Mutex::new(inner)))
    }
}

impl<S: Clone> Clone for State<S> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
