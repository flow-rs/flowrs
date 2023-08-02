use std::sync::{Arc, Mutex, Weak};

use serde::Deserialize;


pub trait ChangeObserver : Send{
    fn on_change(&mut self);
}

#[derive(Clone)]
pub struct Context {
    change_observer: Option<Arc<Mutex<dyn ChangeObserver>>> 
}

impl Context {
    pub fn on_change(&self){
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

    fn on_init(&self);
    fn on_ready(&self);
    fn on_shutdown(&self);
    fn update(&self) -> Result<(), UpdateError>;
}

pub fn to_shared<T : Node>(value: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(value))
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
