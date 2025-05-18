use std::{error::Error, fmt, str::FromStr};

use crate::comm::messages::Message;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use async_trait::async_trait;
use flowrs_package::flow_package::package::Type;

#[cfg(not(target_arch = "wasm32"))]
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;

/// Communicator ==================================================================================
#[async_trait]
pub trait Communicator<D>
where
    D: fmt::Debug,
    D: FromStr,
    D: Send + 'static,
{
    async fn send(&mut self, message: Message<D>) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn Error + Send + Sync>>;

    async fn try_receive(&mut self) -> Result<Option<Message<D>>, Box<dyn Error + Send + Sync>>;

    fn clone_send(&self) -> Self
    where
        Self: Sized;

    fn move_recv(&mut self) -> Result<Self, Box<dyn Error + Send + Sync>>
    where
        Self: Sized;

    async fn connect_send(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;

    async fn connect_recv(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn Error + Send + Sync>>;
}

/// NodeCommunicator ==============================================================================

#[derive(PartialEq)]
pub enum NodeCommunicator<D>
where
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    ThreadComm(ThreadCommunicator<D>),
    //ProcessComm(ProcessCommunicator),
    NetworkComm(NetworkCommunicator<D>),
}

pub const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
pub const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";
pub const SETUP_COMMUNICATION_PREFIX: &str = "[[MESSAGE]: SetupCommunication]";
pub const SETUP_COMMUNICATION_COMM: &str = "[Communicator:[";
pub const SETUP_COMMUNICATION_TYPE: &str = "], Type:[";
pub const DEBUG: &str = "[[MESSAGE]: [DEBUG]>]";
pub const DATA: &str = "[[MESSAGE]: [DATA]>]";

const PATTERNS: &[&str] = &[
    START_EXECUTION,
    STOP_EXECUTION,
    DEBUG,
    DATA,
    SETUP_COMMUNICATION_COMM,
    SETUP_COMMUNICATION_PREFIX,
    SETUP_COMMUNICATION_TYPE,
];

impl<D> NodeCommunicator<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    //use aho_corasick crate to match string prefix, see https://stackoverflow.com/a/64322185
    fn aho_corasick_match<T: AsRef<[u8]>>(ac: &AhoCorasick, v: T) -> Option<&'static str> {
        ac.find(&v).map(|m| PATTERNS[m.pattern()])
    }
    pub fn from_str(s: &str) -> Option<Self> {
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(PATTERNS)
            .unwrap();

        match Self::aho_corasick_match(&ac, s) {
            //match()
            _ => None,
        }
    }
}

impl<D> fmt::Debug for NodeCommunicator<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCommunicator::ThreadComm(comm) => write!(f, "{:?}", comm),
            NodeCommunicator::NetworkComm(comm) => write!(f, "{:?}", comm),
            //NodeCommunicator::ProcessComm(comm) => write!(f, "{:?}", comm),
        }
    }
}

impl<D> fmt::Display for NodeCommunicator<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCommunicator::ThreadComm(comm) => write!(f, "{}", comm),
            NodeCommunicator::NetworkComm(comm) => write!(f, "{}", comm),
            //NodeCommunicator::ProcessComm(comm) => write!(f, "{}", comm),
        }
    }
}

#[async_trait]
impl<D> Communicator<D> for NodeCommunicator<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    async fn send(
        &mut self,
        message: Message<D>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            NodeCommunicator::ThreadComm(comm) => comm.send(message).await,
            NodeCommunicator::NetworkComm(comm) => comm.send(message).await,
        }
    }

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            NodeCommunicator::ThreadComm(comm) => comm.receive().await,
            NodeCommunicator::NetworkComm(comm) => comm.receive().await,
        }
    }

    async fn try_receive(
        &mut self,
    ) -> Result<Option<Message<D>>, Box<dyn std::error::Error + Send + Sync>> {
        match self {
            NodeCommunicator::ThreadComm(comm) => comm.try_receive().await,
            NodeCommunicator::NetworkComm(comm) => comm.try_receive().await,
        }
    }

    fn clone_send(&self) -> Self
    where
        Self: Sized,
    {
        match self {
            NodeCommunicator::ThreadComm(comm) => NodeCommunicator::ThreadComm(comm.clone_send()),
            NodeCommunicator::NetworkComm(comm) => NodeCommunicator::NetworkComm(comm.clone_send()),
        }
    }

    fn move_recv(&mut self) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: Sized,
    {
        match self {
            NodeCommunicator::ThreadComm(comm) => {
                let new_comm = comm.move_recv()?;
                Ok(NodeCommunicator::ThreadComm(new_comm))
            }
            NodeCommunicator::NetworkComm(comm) => {
                let new_comm = comm.move_recv()?;
                Ok(NodeCommunicator::NetworkComm(new_comm))
            }
        }
    }

    async fn connect_send(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            NodeCommunicator::ThreadComm(comm) => comm.connect_send(addr, port).await,
            NodeCommunicator::NetworkComm(comm) => comm.connect_send(addr, port).await,
        }
    }

    async fn connect_recv(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        match self {
            NodeCommunicator::ThreadComm(comm) => comm.connect_recv(addr, port).await,
            NodeCommunicator::NetworkComm(comm) => comm.connect_recv(addr, port).await,
        }
    }
}

/// CommWrapper ===================================================================================
#[derive(PartialEq, Debug)]
pub struct CommWrapper<D>
where
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    pub communicator: NodeCommunicator<D>,
    pub node_type: Type,
}
