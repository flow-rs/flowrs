use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

use std::collections::VecDeque;

use crate::comm::communication::{Communicator, NodeCommunicator};
use crate::comm::data::DataWrapper;
use crate::comm::messages::Message;
#[cfg(not(target_arch = "wasm32"))]
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use crate::node::{Node, ReceiveError, SendError};
use async_trait::async_trait;
#[cfg(not(target_arch = "wasm32"))]
use futures::executor::block_on;
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Handle;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_futures::spawn_local;

#[derive(Debug)]
pub struct Edge<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    communicator: NodeCommunicator<D>,
    send_queue: Vec<D>,
    buffer: Option<D>,
    is_ready: bool,
}

impl<D> Edge<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    pub fn new(communicator: NodeCommunicator<D>) -> Self {
        Self {
            communicator: communicator,
            buffer: None,
            is_ready: false,
            send_queue: Vec::new(),
        }
    }

    pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
        debug_assert!(
            self.buffer.is_none(),
            "get_communicator_mut() called after runtime started"
        );
        match &mut self.communicator {
            NodeCommunicator::ThreadComm(comm) => Some(comm),
            _ => None,
        }
    }

    pub fn enqueue_send(&mut self, data: D) -> Result<(), SendError> {
        self.send_queue.push(data);
        Ok(())
    }

    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        self.enqueue_send(data)
    }

    pub async fn flush(&mut self) -> Result<(), SendError> {
        for data in self.send_queue.drain(..) {
            let data_wrapper = DataWrapper::<D>::new(data);
            let msg = Message::<D>::Data(data_wrapper);

            match &mut self.communicator {
                NodeCommunicator::ThreadComm(comm) => comm
                    .send(msg)
                    .await
                    .map_err(|e| SendError::Other(anyhow::anyhow!(e)))?,
                #[cfg(not(target_arch = "wasm32"))]
                NodeCommunicator::NetworkComm(comm) => comm
                    .send(msg)
                    .await
                    .map_err(|e| SendError::Other(anyhow::anyhow!(e)))?,
            }
        }

        Ok(())
    }

    pub fn set_buffer(&mut self, val: Option<D>) {
        self.buffer = val;
        self.is_ready = self.buffer.is_some();
    }

    pub fn has_data(&self) -> bool {
        self.buffer.is_some()
    }

    pub fn take(&mut self) -> Option<D> {
        let val = self.buffer.take();
        self.is_ready = false;
        val
    }

    pub async fn poll_and_buffer(&mut self) -> Result<(), ReceiveError<D>> {
        match &self.communicator {
            NodeCommunicator::ThreadComm(_) => {
                tracing::debug!("[Edge] Using ThreadCommunicator");
            }
            #[cfg(not(target_arch = "wasm32"))]
            NodeCommunicator::NetworkComm(_) => {
                tracing::debug!("[Edge] Using NetworkCommunicator");
            }
        }

        if self.buffer.is_some() {
            self.is_ready = true;
            tracing::debug!("[Edge] buffer already filled");
            return Ok(());
        }

        match self.try_message().await {
            Ok(Some(Message::Data(data))) => {
                let val = data.get_data();
                self.set_buffer(Some(val));
                tracing::debug!("[Edge] buffer set!");
                Ok(())
            }
            Ok(Some(msg)) => {
                tracing::debug!("[Edge] received control msg");
                Err(ReceiveError::ControlMessage(msg))
            }
            Ok(None) => {
                self.is_ready = false;
                tracing::debug!("[Edge] nothing received");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn try_message(&mut self) -> Result<Option<Message<D>>, ReceiveError<D>> {
        let res = match &mut self.communicator {
            NodeCommunicator::ThreadComm(comm) => comm.try_receive().await,
            #[cfg(not(target_arch = "wasm32"))]
            NodeCommunicator::NetworkComm(comm) => comm.try_receive().await,
        };

        match res {
            Ok(msg_option) => Ok(msg_option),
            Err(err) => Err(ReceiveError::Other(anyhow::anyhow!(err.to_string()))),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<D> Edge<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    // pub fn send(&self, data: D) -> Result<(), SendError> {
    //     let data_wrapper = DataWrapper::<D>::new(data);
    //     let msg = Message::Data(data_wrapper);

    //     let mut guard = block_on(self.communicator.lock());
    //     match &mut *guard {
    //         NodeCommunicator::ThreadComm(comm) => {
    //             block_on(comm.send(msg)).map_err(|e| SendError::Other(anyhow::anyhow!(e)))
    //         }
    //         NodeCommunicator::NetworkComm(comm) => {
    //             block_on(comm.send(msg)).map_err(|e| SendError::Other(anyhow::anyhow!(e)))
    //         }
    //     }
    // }

    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        let fut = match &mut self.communicator {
            NodeCommunicator::ThreadComm(comm) => comm.try_receive(),
            #[cfg(not(target_arch = "wasm32"))]
            NodeCommunicator::NetworkComm(comm) => comm.try_receive(),
        };

        match block_on(fut) {
            Ok(Some(Message::Data(data))) => Ok(Some(data.get_data())),
            Ok(Some(msg)) => Err(ReceiveError::ControlMessage(msg)),
            Ok(None) => Ok(None),
            Err(err) => Err(ReceiveError::Other(anyhow::anyhow!(err))),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<D> Edge<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    // pub fn send(&mut self, data: D) -> Result<(), SendError> {
    //     let comm = Arc::clone(&self.communicator);
    //     spawn_local(async move {
    //         let msg = Message::<D>::Data(DataWrapper::new(data));
    //         let mut guard = comm.lock().await; // <-- FIXED

    //         if let NodeCommunicator::ThreadComm(comm) = &mut *guard {
    //             let _ = comm.send(msg).await;
    //         }
    //     });
    //     Ok(())
    // }

    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        Ok(None)
    }

    #[cfg(target_arch = "wasm32")]
    pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        match self.try_message().await? {
            Some(Message::Data(data)) => Ok(Some(data.get_data())),
            Some(msg) => Err(ReceiveError::ControlMessage(msg)),
            None => Ok(None),
        }
    }
}

#[derive(Debug)]
pub struct Input<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    pub edge: Edge<D>,
}

#[derive(Debug)]
pub struct Output<D>
where
    D: Clone + fmt::Debug + FromStr + Send + 'static,
{
    pub edge: Edge<D>,
    pub pending: VecDeque<Message<D>>,
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

    pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
        self.edge.get_communicator_mut()
    }

    /// Adds the message to the outgoing buffer (async flush will be called later)
    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        self.edge.enqueue_send(data)
    }

    /// Blocking-style non-async receive for native use
    #[cfg(not(target_arch = "wasm32"))]
    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next()
    }

    /// Async-style receive for wasm
    #[cfg(target_arch = "wasm32")]
    pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next_async().await
    }

    /// Flushes the buffered outgoing data (must be called after `on_update`)
    pub async fn flush(&mut self) -> Result<(), SendError> {
        self.edge.flush().await
    }

    pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
        self.edge = Edge::new(communicator);
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
            pending: VecDeque::new(),
        }
    }

    pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
        self.edge.get_communicator_mut()
    }

    /// Queues a value for sending (actual send will happen on `flush()`)
    pub fn send(&mut self, data: D) -> Result<(), SendError> {
        self.edge.enqueue_send(data)
    }

    /// Blocking-style non-async receive for native use
    #[cfg(not(target_arch = "wasm32"))]
    pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next()
    }

    /// Async-style receive for wasm
    #[cfg(target_arch = "wasm32")]
    pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
        self.edge.next_async().await
    }

    /// Flushes queued data (should be called at end of `on_update`)
    pub async fn flush(&mut self) -> Result<(), SendError> {
        self.edge.flush().await
    }

    pub fn with_thread_comm<F, R>(&mut self, f: F) -> Option<R>
    where
        F: FnOnce(&mut ThreadCommunicator<D>) -> R,
    {
        match &mut self.edge.communicator {
            NodeCommunicator::ThreadComm(comm) => Some(f(comm)),
            _ => None,
        }
    }

    pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
        self.edge = Edge::new(communicator);
    }

    // pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
    //     match &mut self.edge.communicator {
    //         NodeCommunicator::ThreadComm(comm) => Some(comm),
    //         _ => None,
    //     }
    // }

    pub fn from_communicator(communicator: ThreadCommunicator<D>) -> Self {
        Self {
            edge: Edge::new(NodeCommunicator::ThreadComm(communicator)),
            pending: VecDeque::new(),
        }
    }
}

#[async_trait]
pub trait EdgeTrait<D>: Sized
where
    D: Clone + Send + fmt::Debug + FromStr + 'static,
{
    fn new_local() -> Self;
    async fn new_network() -> Self;
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

    #[cfg(not(target_arch = "wasm32"))]
    async fn new_network() -> Self {
        Input::new(NodeCommunicator::NetworkComm(
            NetworkCommunicator::new().await.expect("should construct"),
        ))
    }

    #[cfg(target_arch = "wasm32")]
    async fn new_network() -> Self {
        panic!("new_network is not supported on wasm");
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

    #[cfg(not(target_arch = "wasm32"))]
    async fn new_network() -> Self {
        Output::new(NodeCommunicator::NetworkComm(
            NetworkCommunicator::new().await.expect("should construct"),
        ))
    }

    #[cfg(target_arch = "wasm32")]
    async fn new_network() -> Self {
        panic!("new_network is not supported on wasm");
    }
}

// use std::fmt;
// use std::str::FromStr;

// use crate::comm::communication::{Communicator, NodeCommunicator};
// use crate::comm::data::DataWrapper;
// use crate::comm::messages::Message;
// #[cfg(not(target_arch = "wasm32"))]
// use crate::comm::network_communicator::NetworkCommunicator;
// use crate::comm::thread_communicator::ThreadCommunicator;
// use crate::node::{Node, ReceiveError, SendError};
// use async_trait::async_trait;
// use futures::executor::block_on;
// use tokio::runtime::Handle;

// #[derive(Debug)]
// pub struct Edge<D>
// where
//     D: Clone,
//     D: fmt::Debug,
//     D: FromStr,
//     D: Send + 'static,
// {
//     communicator: NodeCommunicator<D>,
//     pub buffer: Option<D>,
//     is_ready: bool,
// }

// impl<D> Edge<D>
// where
//     D: Clone,
//     D: fmt::Debug,
//     D: FromStr,
//     D: Send + 'static,
// {
//     pub fn new(communicator: NodeCommunicator<D>) -> Self {
//         Self {
//             communicator,
//             buffer: None,
//             is_ready: false,
//         }
//     }

//     pub fn set_buffer(&mut self, val: Option<D>) {
//         self.buffer = val;
//         self.is_ready = self.buffer.is_some();
//     }

//     pub fn has_data(&self) -> bool {
//         self.buffer.is_some()
//     }

//     pub fn take(&mut self) -> Option<D> {
//         let val = self.buffer.take();
//         self.is_ready = false;
//         val
//     }

//     /// Polls the underlying communicator once and updates the internal buffer accordingly.
//     pub async fn poll_and_buffer(&mut self) -> Result<(), ReceiveError<D>> {
//         match &self.communicator {
//             NodeCommunicator::ThreadComm(_) => tracing::debug!("[Edge] Using ThreadCommunicator"),
//             #[cfg(not(target_arch = "wasm32"))]
//             NodeCommunicator::NetworkComm(_) => tracing::debug!("[Edge] Using NetworkCommunicator"),
//         }
//         // If we already have a buffered message, don't re-poll
//         if self.buffer.is_some() {
//             self.is_ready = true;
//             tracing::debug!("[Edge] buffer already filled");
//             return Ok(());
//         }

//         match self.try_message().await {
//             Ok(Some(Message::Data(data))) => {
//                 let val = data.get_data();
//                 self.set_buffer(Some(val));
//                 tracing::debug!("[Edge] buffer set!");
//                 Ok(())
//             }

//             Ok(Some(msg)) => {
//                 // Pass control message upwards
//                 tracing::debug!("[Edge] error");
//                 Err(ReceiveError::ControlMessage(msg))
//             }

//             Ok(None) => {
//                 self.is_ready = false;
//                 tracing::debug!("[Edge] nothing received");
//                 Ok(())
//             }

//             Err(e) => Err(e),
//         }
//     }

//     // Try to receive any message over the Edge. Use this function to retrieve control messages
//     pub async fn try_message(&mut self) -> Result<Option<Message<D>>, ReceiveError<D>> {
//         let res = match &mut self.communicator {
//             NodeCommunicator::ThreadComm(communicator) => communicator.try_receive().await,
//             #[cfg(not(target_arch = "wasm32"))]
//             NodeCommunicator::NetworkComm(communicator) => communicator.try_receive().await,
//         };
//         match res {
//             Ok(msg_option) => match msg_option {
//                 Some(msg) => Ok(Some(msg)),
//                 None => Ok(None),
//             },
//             Err(err) => Err(ReceiveError::Other(anyhow::Error::msg(format!("{}", err)))),
//         }
//     }
// }

// #[cfg(not(target_arch = "wasm32"))]
// impl<D> Edge<D>
// where
//     D: Clone,
//     D: fmt::Debug,
//     D: FromStr,
//     D: Send + 'static,
// {
//     pub fn send(&mut self, data: D) -> Result<(), SendError> {
//         let data_wrapper = DataWrapper::<D>::new(data);
//         let msg = Message::<D>::Data(data_wrapper);

//         match &mut self.communicator {
//             NodeCommunicator::ThreadComm(communicator) => block_on(communicator.send(msg))
//                 .map_err(|e| SendError::Other(anyhow::Error::msg(format!("{}", e)))),
//             NodeCommunicator::NetworkComm(communicator) => block_on(communicator.send(msg))
//                 .map_err(|e| SendError::Other(anyhow::Error::msg(format!("{}", e)))),
//         }
//     }

//     pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         let fut = match &mut self.communicator {
//             NodeCommunicator::ThreadComm(comm) => comm.try_receive(),
//             NodeCommunicator::NetworkComm(comm) => comm.try_receive(),
//         };

//         match Handle::current().block_on(fut) {
//             Ok(Some(Message::Data(data))) => Ok(Some(data.get_data())),
//             Ok(Some(msg)) => Err(ReceiveError::ControlMessage(msg)),
//             Ok(None) => Ok(None),
//             Err(err) => Err(ReceiveError::Other(anyhow::Error::msg(err.to_string()))),
//         }
//     }
// }

// #[cfg(target_arch = "wasm32")]
// impl<D> Edge<D>
// where
//     D: Clone,
//     D: fmt::Debug,
//     D: FromStr,
//     D: Send + 'static,
// {
//     pub async fn send_async(&mut self, data: D) -> Result<(), SendError> {
//         let data_wrapper = DataWrapper::<D>::new(data);
//         let msg = Message::<D>::Data(data_wrapper);

//         match &mut self.communicator {
//             NodeCommunicator::ThreadComm(communicator) => communicator
//                 .send(msg)
//                 .await
//                 .map_err(|e| SendError::Other(anyhow::Error::msg(format!("{}", e)))),
//         }
//     }

//     pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         let fut = match &mut self.communicator {
//             NodeCommunicator::ThreadComm(comm) => comm.try_receive(),
//         };

//         match fut.await {
//             Ok(Some(Message::Data(data))) => Ok(Some(data.get_data())),
//             Ok(Some(msg)) => Err(ReceiveError::ControlMessage(msg)),
//             Ok(None) => Ok(None),
//             Err(err) => Err(ReceiveError::Other(anyhow::Error::msg(err.to_string()))),
//         }
//     }
// }

// #[derive(Debug)]
// pub struct Input<D>
// where
//     D: Clone + fmt::Debug + FromStr + Send + 'static,
// {
//     pub edge: Edge<D>,
// }

// #[derive(Debug)]
// pub struct Output<D>
// where
//     D: Clone + fmt::Debug + FromStr + Send + 'static,
// {
//     pub edge: Edge<D>,
// }

// #[async_trait]
// pub trait EdgeTrait<D>: Sized
// where
//     D: Clone + Send + fmt::Debug + FromStr + 'static,
// {
//     fn new_local() -> Self;
//     async fn new_network() -> Self;
// }

// impl<D> Input<D>
// where
//     D: Clone + fmt::Debug + FromStr + Send + 'static,
// {
//     pub fn new(communicator: NodeCommunicator<D>) -> Self {
//         Self {
//             edge: Edge::new(communicator),
//         }
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     pub fn send(&mut self, data: D) -> Result<(), SendError> {
//         self.edge.send(data)
//     }

//     #[cfg(target_arch = "wasm32")]
//     pub async fn send_async(&mut self, data: D) -> Result<(), SendError> {
//         self.edge.send_async(data).await
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         self.edge.next()
//     }

//     #[cfg(target_arch = "wasm32")]
//     pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         self.edge.next_async().await
//     }

//     /// Set a new communicator
//     pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
//         self.edge = Edge::new(communicator);
//     }

//     pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
//         match &mut self.edge.communicator {
//             NodeCommunicator::ThreadComm(comm) => Some(comm),
//             _ => None,
//         }
//     }

//     pub fn from_communicator(communicator: ThreadCommunicator<D>) -> Self {
//         Self {
//             edge: Edge::new(NodeCommunicator::ThreadComm(communicator)),
//         }
//     }

//     pub fn edge_mut(&mut self) -> &mut Edge<D> {
//         &mut self.edge
//     }
// }

// impl<D> Output<D>
// where
//     D: Clone + fmt::Debug + FromStr + Send + 'static,
// {
//     pub fn new(communicator: NodeCommunicator<D>) -> Self {
//         Self {
//             edge: Edge::new(communicator),
//         }
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     pub fn send(&mut self, data: D) -> Result<(), SendError> {
//         self.edge.send(data)
//     }

//     #[cfg(target_arch = "wasm32")]
//     pub async fn send_async(&mut self, data: D) -> Result<(), SendError> {
//         self.edge.send_async(data).await
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     pub fn next(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         self.edge.next()
//     }

//     #[cfg(target_arch = "wasm32")]
//     pub async fn next_async(&mut self) -> Result<Option<D>, ReceiveError<D>> {
//         self.edge.next_async().await
//     }

//     /// Set a new communicator
//     pub fn set_communicator(&mut self, communicator: NodeCommunicator<D>) {
//         self.edge = Edge::new(communicator);
//     }

//     pub fn get_communicator_mut(&mut self) -> Option<&mut ThreadCommunicator<D>> {
//         match &mut self.edge.communicator {
//             NodeCommunicator::ThreadComm(comm) => Some(comm),
//             _ => None,
//         }
//     }

//     pub fn from_communicator(communicator: ThreadCommunicator<D>) -> Self {
//         Self {
//             edge: Edge::new(NodeCommunicator::ThreadComm(communicator)),
//         }
//     }
// }

// #[async_trait]
// impl<D> EdgeTrait<D> for Input<D>
// where
//     D: Clone + Send + fmt::Debug + FromStr + 'static,
// {
//     fn new_local() -> Self {
//         Input::new(NodeCommunicator::ThreadComm(
//             ThreadCommunicator::<D>::new().expect("should construct"),
//         ))
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     async fn new_network() -> Self {
//         Input::new(NodeCommunicator::NetworkComm(
//             NetworkCommunicator::new().await.expect("should construct"),
//         ))
//     }

//     #[cfg(target_arch = "wasm32")]
//     async fn new_network() -> Self {
//         panic!("new_network is not supported on wasm");
//     }
// }

// #[async_trait]
// impl<D> EdgeTrait<D> for Output<D>
// where
//     D: Clone + Send + fmt::Debug + FromStr + 'static,
// {
//     fn new_local() -> Self {
//         Output::new(NodeCommunicator::ThreadComm(
//             ThreadCommunicator::<D>::new().expect("should construct"),
//         ))
//     }

//     #[cfg(not(target_arch = "wasm32"))]
//     async fn new_network() -> Self {
//         Output::new(NodeCommunicator::NetworkComm(
//             NetworkCommunicator::new().await.expect("should construct"),
//         ))
//     }

//     #[cfg(target_arch = "wasm32")]
//     async fn new_network() -> Self {
//         panic!("new_network is not supported on wasm");
//     }
// }

// /// A [Node] that implements the [RuntimeConnectable] trait.
// //pub trait RuntimeNode: Node + RuntimeConnectable {}
// pub trait RuntimeNode: Node {}

// //impl<T> RuntimeNode for T where T: Node + RuntimeConnectable {}
// impl<T> RuntimeNode for T where T: Node {}

// #[cfg(test)]
// mod test {
//     use super::*;
//     use crate::comm::thread_communicator::ThreadCommunicator;

//     #[tokio::test]
//     async fn test_send() {
//         // Create an edge
//         let communicator =
//             ThreadCommunicator::<String>::new().expect("creation of a ThreadCommunicator object");
//         let mut edge = Edge::new(NodeCommunicator::ThreadComm(communicator));

//         // Send something
//         let test_data = "Hello World!".to_string();
//         let res = edge.send(test_data);

//         // Assert Result
//         assert!(res.is_ok());
//     }

//     #[tokio::test]
//     async fn test_next() {
//         // Create an edge
//         let communicator =
//             ThreadCommunicator::<String>::new().expect("creation of a ThreadCommunicator object");
//         let mut edge = Edge::new(NodeCommunicator::ThreadComm(communicator));

//         // Send something
//         let test_data = "Hello World!".to_string();
//         let _ = edge.send(test_data.clone());

//         // Try to receive the message
//         let res = edge.next();

//         // Assert result
//         assert!(res.is_ok());
//         let msg = res.unwrap().unwrap();
//         assert_eq!(msg, test_data);
//     }
// }
