use std::sync::{Arc, Mutex};

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Context {}

pub trait Node {
    fn name(&self) -> &str;
    fn on_init(&self);
    fn on_ready(&self);
    fn on_shutdown(&self);
    fn update(&self) -> Result<(), UpdateError>;
}

#[derive(Debug)]
pub struct SequenceError {
    pub node: String,
    pub message: String,
}

#[derive(Debug)]
pub enum UpdateError {
    SequenceError(SequenceError),
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
