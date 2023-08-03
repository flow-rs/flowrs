use std::sync::{Arc, Mutex};
use thiserror::Error;
use anyhow::Result;

pub trait ChangeObserver: Send {
    fn on_change(&mut self);
}

#[derive(Clone)]
pub struct Context {
    change_observer: Option<Arc<Mutex<dyn ChangeObserver>>>,
}

impl Context {
    pub fn on_change(&self) {
        if let Some(so) = &self.change_observer {
            so.lock().unwrap().on_change();
        }
    }

    pub fn new() -> Self {
        Self {
            change_observer: None,
        }
    }

    pub fn set_observer(&mut self, observer: Arc<Mutex<dyn ChangeObserver>>) {
        self.change_observer = Some(observer);
    }
}

pub trait Node : Send + 'static {
    fn name(&self) -> &str;

    fn on_init(&self) -> Result<(), InitError>;
    fn on_ready(&self) -> Result<(), ReadyError>;
    fn on_shutdown(&self) -> Result<(), ShutdownError>;
    fn update(&self) -> Result<(), UpdateError>;
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
pub enum UpdateError {

    #[error("Sequence error for node {node:?}. Message: {message:?}")]
    SequenceError {
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
