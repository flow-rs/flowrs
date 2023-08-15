use serde::{Serialize, Serializer, Deserialize, Deserializer, de::IgnoredAny};

use crate::node::{ChangeObserver, Node, ReceiveError, SendError};
use std::{
    any::Any,
    rc::Rc,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    }, fmt::Debug,
};

#[derive(Debug)]
pub struct Edge<I> {
    sender: Sender<I>,
    receiver: Option<Receiver<I>>,
}

impl<I> Clone for Edge<I> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: None,
        }
    }
}

impl<I> Edge<I> {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver: Some(receiver),
        }
    }

    pub fn send(&self, elem: I) -> Result<(), SendError> {
        match self.sender.send(elem) {
            Ok(_) => Ok(()),
            Err(err) => Err(SendError::Other(anyhow::Error::msg(err.to_string()))),
        }
    }

    pub fn next(&self) -> Result<I, ReceiveError> {
        let res = self
            .receiver
            .as_ref()
            .expect("Only the Node that created this edge can receive from it.")
            .try_recv();
        match res {
            Ok(i) => Ok(i),
            Err(err) => Err(ReceiveError::Other(err.into()))
        }
    }
}

pub type Input<I> = Edge<I>;

impl<T> Serialize for Edge<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
        serializer.serialize_unit()
    }
}

impl<'de, T> Deserialize<'de> for Edge<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de> {
        deserializer.deserialize_any(IgnoredAny).unwrap();
        Ok(Self::new())
    }
}

#[derive(Clone)]
pub struct Output<T> {
    edge: Arc<Mutex<Option<Edge<T>>>>,
    change_notifier: Option<Sender<bool>>,
}

impl<T> Serialize for Output<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer {
        serializer.serialize_unit()
    }
}

impl<'de, T> Deserialize<'de> for Output<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de> {
        deserializer.deserialize_any(IgnoredAny).unwrap();
        Ok(Self::new(None))
    }
}

impl<O> Output<O> {
    pub fn new(change_observer: Option<&ChangeObserver>) -> Self {
        let change_notifier = change_observer.map(|observer| observer.notifier.clone());
        Self {
            edge: Arc::new(Mutex::new(None)),
            change_notifier: change_notifier,
        }
    }

    pub fn set_sender(mut self, edge: Edge<O>) -> Self {
        self.edge = Arc::new(Mutex::new(Some(edge)));
        self
    }

    pub fn set_observer(mut self, change_observer: &ChangeObserver) -> Self {
        let change_notifier = change_observer.notifier.clone();
        self.change_notifier = Some(change_notifier);
        self
    }

    pub fn send(&mut self, elem: O) -> Result<(), SendError> {
        let _res = self
            .edge
            .lock()
            .unwrap()
            .as_mut()
            .ok_or(SendError::Other(anyhow::Error::msg("Failed to send item to output")))?
            .send(elem);

        if let Some(cn) = &self.change_notifier { 
            let _ = cn.send(true);
        }

        Ok(())
    }

    pub fn set(&mut self, edge: Edge<O>) {
        let _ = self.edge.lock().unwrap().insert(edge);
    }
}

pub fn connect<I: Clone>(mut lhs: Output<I>, rhs: Input<I>) {
    lhs.set(rhs)
}

pub trait RuntimeConnectable {
    fn input_at(&self, index: usize) -> Rc<dyn Any>;
    fn output_at(&self, index: usize) -> Rc<dyn Any>;
}
pub trait RuntimeNode: Node + RuntimeConnectable {}
impl<T> RuntimeNode for T where T: Node + RuntimeConnectable {}
