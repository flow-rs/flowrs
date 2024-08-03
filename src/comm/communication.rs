use std::fmt;

use crate::comm::messages::Message;

use async_trait::async_trait;
use flowrs_package::flow_package::package::Type;

use super::{process_communicator::ProcessCommunicator, thread_communicator::ThreadCommunicator};

/// Communicator ==================================================================================
#[async_trait]
pub trait Communicator {
    async fn send(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>>;
    async fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>>;
}

/// NodeCommunicator ==============================================================================

#[derive(PartialEq)]
pub enum NodeCommunicator {
    ThreadComm(ThreadCommunicator),
    ProcessComm(ProcessCommunicator),
    //NetworkComm(NetworkCommunicator),
}

impl fmt::Debug for NodeCommunicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCommunicator::ThreadComm(comm) => write!(f, "{:?}", comm),
            NodeCommunicator::ProcessComm(comm) => write!(f, "{:?}", comm),
        }
    }
}

impl fmt::Display for NodeCommunicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCommunicator::ThreadComm(comm) => write!(f, "{}", comm),
            NodeCommunicator::ProcessComm(comm) => write!(f, "{}", comm),
        }
    }
}

/// CommWrapper ===================================================================================
#[derive(PartialEq, Debug)]
pub struct CommWrapper {
    pub communicator: NodeCommunicator,
    pub node_type: Type,
}
