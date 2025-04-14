use std::fmt;
use std::str::FromStr;

use crate::comm::communication::{Communicator, NodeCommunicator};
use crate::comm::data::DataWrapper;
use crate::comm::messages::Message;
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use crate::node::{Node, ReceiveError, SendError};
use async_trait::async_trait;
use futures::executor::block_on;
use tokio::runtime::Handle;

#[derive(Debug)]
pub struct Edge<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send + 'static,
{
    communicator: NodeCommunicator<D>,
    buffer: Option<D>,
}

impl<D> Edge<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send + 'static,
{
    pub fn new(communicator: NodeCommunicator<D>) -> Self {
        Self {
            communicator,
            buffer: None,
        }
    }

    pub fn set_buffer(&mut self, val: Option<D>) {
        self.buffer = val;
    }

    pub fn has_data(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn take(&mut self) -> Option<D> {
        self.buffer.take()
    }

    /// Polls the underlying communicator once and updates the internal buffer accordingly.
    pub async fn poll_and_buffer(&mut self) -> Result<(), ReceiveError<D>> {
        match self.try_message().await {
            Ok(Some(Message::Data(data))) => {
                self.set_buffer(Some(data.get_data()));
                Ok(())
            }
            Ok(Some(msg)) => Err(ReceiveError::ControlMessage(msg)),
            Ok(None) => {
                // No message yet, explicitly store `None`
                self.set_buffer(None);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    // Send a single data point over the edge
    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        let data_wrapper = DataWrapper::<D>::new(data);
        let msg = Message::<D>::Data(data_wrapper);
        match &mut self.communicator {
            NodeCommunicator::ThreadComm(communicator) => block_on(communicator.send(msg))
                .map_err(|e| SendError::Other(anyhow::Error::msg(format!("{}", e)))),
            NodeCommunicator::NetworkComm(communicator) => block_on(communicator.send(msg))
                .map_err(|e| SendError::Other(anyhow::Error::msg(format!("{}", e)))),
        }
    }

    // Receive a single data point over the edge
    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        let fut = match &mut self.communicator {
            NodeCommunicator::ThreadComm(comm) => comm.try_receive(),
            NodeCommunicator::NetworkComm(comm) => comm.try_receive(),
        };

        match Handle::current().block_on(fut) {
            Ok(Some(Message::Data(data))) => Ok(Some(data.get_data())),
            Ok(Some(msg)) => Err(ReceiveError::ControlMessage(msg)),
            Ok(None) => Ok(None), // No message yet — this is non-blocking behavior
            Err(err) => Err(ReceiveError::Other(anyhow::Error::msg(err.to_string()))),
        }
    }

    // Try to receive any message over the Edge. Use this function to retrieve control messages
    pub async fn try_message(&mut self) -> Result<Option<Message<D>>, ReceiveError<D>> {
        let res = match &mut self.communicator {
            NodeCommunicator::ThreadComm(communicator) => communicator.try_receive().await,
            NodeCommunicator::NetworkComm(communicator) => communicator.try_receive().await,
        };
        match res {
            Ok(msg_option) => match msg_option {
                Some(msg) => Ok(Some(msg)),
                None => Ok(None),
            },
            Err(err) => Err(ReceiveError::Other(anyhow::Error::msg(format!("{}", err)))),
        }
    }
}

#[derive(Debug)]
pub struct Input<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    edge: Edge<D>,
}

#[derive(Debug)]
pub struct Output<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    edge: Edge<D>,
}

// /// A node's input implemented as an [Edge] type.
// pub type Input<D> = Edge<D>;

// /// A node's input implemented as an [Edge] type.
// pub type Output<D> = Edge<D>;

// /// Marker traits for inputs and outputs
// /// (mainly to avoid making them separate types)
// pub trait IsInput {}
// pub trait IsOutput {}
// impl<D> IsInput for Edge<D> where D: Clone + Send + std::fmt::Debug + std::str::FromStr + 'static {}
// impl<D> IsOutput for Edge<D> where D: Clone + Send + std::fmt::Debug + std::str::FromStr + 'static {}

#[async_trait]
pub trait EdgeTrait<D>: Sized
where
    D: Clone + Send + fmt::Debug + FromStr + 'static,
{
    fn new_local() -> Self;
    async fn new_network() -> Self;
}

impl<D> Input<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    pub fn new(communicator: NodeCommunicator<D>) -> Self {
        Self {
            edge: Edge::new(communicator),
        }
    }

    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        self.edge.send(data)
    }

    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next()
    }

    /// Set a new communicator
    pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
        self.edge = Edge::new(communicator);
    }

    pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
        match &mut self.edge.communicator {
            NodeCommunicator::ThreadComm(comm) => Some(comm),
            _ => None,
        }
    }

    pub fn from_communicator(communicator: ThreadCommunicator<D>) -> Self {
        Self {
            edge: Edge::new(NodeCommunicator::ThreadComm(communicator)),
        }
    }

    pub fn edge_mut(&mut self) -> &mut Edge<D> {
        &mut self.edge
    }
}

impl<D> Output<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    pub fn new(communicator: NodeCommunicator<D>) -> Self {
        Self {
            edge: Edge::new(communicator),
        }
    }

    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        self.edge.send(data)
    }

    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next()
    }

    /// Set a new communicator
    pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
        self.edge = Edge::new(communicator);
    }

    pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
        match &mut self.edge.communicator {
            NodeCommunicator::ThreadComm(comm) => Some(comm),
            _ => None,
        }
    }

    pub fn from_communicator(communicator: ThreadCommunicator<D>) -> Self {
        Self {
            edge: Edge::new(NodeCommunicator::ThreadComm(communicator)),
        }
    }
}

#[async_trait]
impl<D> EdgeTrait<D> for Input<D>
where
    D: Clone + Send + fmt::Debug + FromStr + 'static,
{
    fn new_local() -> Self {
        Input::new(NodeCommunicator::ThreadComm(
            ThreadCommunicator::<D>::new().expect("should construct"),
        ))
    }

    async fn new_network() -> Self {
        Input::new(NodeCommunicator::NetworkComm(
            NetworkCommunicator::new().await.expect("should construct"),
        ))
    }
}

#[async_trait]
impl<D> EdgeTrait<D> for Output<D>
where
    D: Clone + Send + fmt::Debug + FromStr + 'static,
{
    fn new_local() -> Self {
        Output::new(NodeCommunicator::ThreadComm(
            ThreadCommunicator::<D>::new().expect("should construct"),
        ))
    }

    async fn new_network() -> Self {
        Output::new(NodeCommunicator::NetworkComm(
            NetworkCommunicator::new().await.expect("should construct"),
        ))
    }
}

// impl<D> EdgeTrait<D> for Input<D>
// where
//     D: Clone + Send + std::fmt::Debug + std::str::FromStr + 'static,
//     Input<D>: IsInput,
// {
//     async fn new_local() -> Self {
//         Input::new(NodeCommunicator::ThreadComm(
//             ThreadCommunicator::<D>::new().expect("should construct"),
//         ))
//     }

//     async fn new_network() -> Self {
//         Input::new(NodeCommunicator::NetworkComm(
//             NetworkCommunicator::new().await.expect("should construct"),
//         ))
//     }
// }

// impl<D> EdgeTrait<D> for Output<D>
// where
//     D: Clone + Send + std::fmt::Debug + std::str::FromStr + 'static,
//     Output<D>: IsOutput,
// {
//     async fn new_local() -> Self {
//         Output::new(NodeCommunicator::ThreadComm(
//             ThreadCommunicator::<D>::new().expect("should construct"),
//         ))
//     }

//     async fn new_network() -> Self {
//         Output::new(NodeCommunicator::NetworkComm(
//             NetworkCommunicator::new().await.expect("should construct"),
//         ))
//     }
// }

// /// This trait is used for a accessing a node's
// /// inputs and outputs by index at runtime.
// pub trait RuntimeConnectable {
//     fn input_at(&self, index: usize) -> Rc<dyn Any>;
//     fn output_at(&self, index: usize) -> Rc<dyn Any>;
// }

/// A [Node] that implements the [RuntimeConnectable] trait.
//pub trait RuntimeNode: Node + RuntimeConnectable {}
pub trait RuntimeNode: Node {}

//impl<T> RuntimeNode for T where T: Node + RuntimeConnectable {}
impl<T> RuntimeNode for T where T: Node {}

#[cfg(test)]
mod test {
    use super::*;
    use crate::comm::thread_communicator::ThreadCommunicator;

    #[tokio::test]
    async fn test_send() {
        // Create an edge
        let communicator =
            ThreadCommunicator::<String>::new().expect("creation of a ThreadCommunicator object");
        let mut edge = Edge::new(NodeCommunicator::ThreadComm(communicator));

        // Send something
        let test_data = "Hello World!".to_string();
        let res = edge.send(test_data);

        // Assert Result
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_next() {
        // Create an edge
        let communicator =
            ThreadCommunicator::<String>::new().expect("creation of a ThreadCommunicator object");
        let mut edge = Edge::new(NodeCommunicator::ThreadComm(communicator));

        // Send something
        let test_data = "Hello World!".to_string();
        let _ = edge.send(test_data.clone());

        // Try to receive the message
        let res = edge.next();

        // Assert result
        assert!(res.is_ok());
        let msg = res.unwrap().unwrap();
        assert_eq!(msg, test_data);
    }
}

// /// An edge defines the connection between two nodes.
// /// It is implemented using a [`std::sync::mpsc::channel`].
// #[derive(Debug)]
// pub struct Edge<I> {
//     /// The producer side (technically there can be multiple producers).
//     sender: Sender<I>,

//     /// The consumer side (only a single consumer).
//     /// Consumers are optional.
//     receiver: Option<Receiver<I>>,
// }

// impl<I> Clone for Edge<I> {
//     fn clone(&self) -> Self {
//         Self {
//             sender: self.sender.clone(),
//             receiver: None,
//         }
//     }
// }

// impl<I> Edge<I> {
//     pub fn new() -> Self {
//         let (sender, receiver) = channel();
//         Self {
//             sender,
//             receiver: Some(receiver),
//         }
//     }

//     pub fn send(&self, elem: I) -> Result<(), SendError> {
//         let payload_bytes = size_of::<I>();

//         match self.sender.send(elem) {
//             Ok(_) => {
//                 counter!("flowrs.node.edge.send.bytes", payload_bytes as u64);
//                 increment_counter!("flowrs.node.edge.send.count");
//                 Ok(())
//             }
//             Err(err) => Err(SendError::Other(anyhow::Error::msg(format!("{}", err)))),
//         }
//     }

//     pub fn next(&self) -> Result<I, ReceiveError> {
//         let res = self
//             .receiver
//             .as_ref()
//             .expect("Only the Node that created this edge can receive from it.")
//             .try_recv();
//         match res {
//             Ok(i) => {
//                 let payload_bytes = size_of::<I>();
//                 counter!("flowrs.node.edge.receive.bytes", payload_bytes as u64);
//                 increment_counter!("flowrs.node.edge.receive.count");
//                 Ok(i)
//             }
//             Err(err) => Err(ReceiveError::Other(err.into())),
//         }
//     }
// }

// /// A node's input implemented as an [Edge] type.
// pub type Input<I> = Edge<I>;

// impl<T> Serialize for Edge<T> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         serializer.serialize_unit()
//     }
// }

// impl<'de, T> Deserialize<'de> for Edge<T> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_any(IgnoredAny).unwrap();
//         Ok(Self::new())
//     }
// }

// /// A node's output.
// #[derive(Clone)]
// pub struct Output<T> {
//     // The (optional) connection to another node's input.
//     edge: Arc<Mutex<Option<Edge<T>>>>,

//     /// Whenever something is written to the output
//     /// (and a change notifier exists), a change notification is sent.
//     change_notifier: Option<Sender<bool>>,
// }

// impl<T> Serialize for Output<T> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         serializer.serialize_unit()
//     }
// }

// impl<'de, T> Deserialize<'de> for Output<T> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         deserializer.deserialize_any(IgnoredAny).unwrap();
//         Ok(Self::new(None))
//     }
// }

// impl<O> Output<O> {
//     pub fn new(change_observer: Option<&ChangeObserver>) -> Self {
//         let change_notifier = change_observer.map(|observer| observer.notifier.clone());
//         Self {
//             edge: Arc::new(Mutex::new(None)),
//             change_notifier: change_notifier,
//         }
//     }

//     pub fn set_sender(mut self, edge: Edge<O>) -> Self {
//         self.edge = Arc::new(Mutex::new(Some(edge)));
//         self
//     }

//     pub fn set_observer(mut self, change_observer: &ChangeObserver) -> Self {
//         let change_notifier = change_observer.notifier.clone();
//         self.change_notifier = Some(change_notifier);
//         self
//     }

//     pub fn send(&mut self, elem: O) -> Result<(), SendError> {
//         let _res = self
//             .edge
//             .lock()
//             .unwrap()
//             .as_mut()
//             .ok_or(SendError::Other(anyhow::Error::msg(
//                 "Failed to send item to output",
//             )))?
//             .send(elem);

//         if let Some(cn) = &self.change_notifier {
//             let _ = cn.send(true);
//         }

//         Ok(())
//     }

//     pub fn set(&mut self, edge: Edge<O>) {
//         let _ = self.edge.lock().unwrap().insert(edge);
//     }
// }

// pub fn connect<I: Clone>(mut lhs: Output<I>, rhs: Input<I>) {
//     lhs.set(rhs)
// }

// /// This trait is used for a accessing a node's
// /// inputs and outputs by index at runtime.
// pub trait RuntimeConnectable {
//     fn input_at(&self, index: usize) -> Rc<dyn Any>;
//     fn output_at(&self, index: usize) -> Rc<dyn Any>;
// }

// /// A [`Node`] that implements the [`RuntimeConnectable`] trait.
// pub trait RuntimeNode: Node + RuntimeConnectable {}
