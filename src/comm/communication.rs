use std::{fmt, str::FromStr};

use crate::comm::messages::Message;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use async_trait::async_trait;
use flowrs_package::flow_package::package::Type;

use super::{
    network_communicator::NetworkCommunicator,
    //process_communicator::ProcessCommunicator,
    thread_communicator::ThreadCommunicator,
};

/// Communicator ==================================================================================
#[async_trait]
pub trait Communicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    async fn send(&mut self, message: Message<D>) -> Result<(), Box<dyn std::error::Error>>;
    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error>>;
    async fn try_receive(&mut self) -> Result<Option<Message<D>>, Box<dyn std::error::Error>>;

    // Functions needed to seperate sending and receiving
    fn clone_send(&self) -> Self
    where
        Self: Sized;
    fn move_recv(&mut self) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized;

    // Functions needed to connect sending and receiving parts
    async fn connect_send(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn connect_recv(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error>>;
}

/// NodeCommunicator ==============================================================================

#[derive(PartialEq)]
pub enum NodeCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    ThreadComm(ThreadCommunicator<D>),
    //ProcessComm(ProcessCommunicator),
    NetworkComm(NetworkCommunicator),
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
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
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
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
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
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCommunicator::ThreadComm(comm) => write!(f, "{}", comm),
            NodeCommunicator::NetworkComm(comm) => write!(f, "{}", comm),
            //NodeCommunicator::ProcessComm(comm) => write!(f, "{}", comm),
        }
    }
}

/// CommWrapper ===================================================================================
#[derive(PartialEq, Debug)]
pub struct CommWrapper<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    pub communicator: NodeCommunicator<D>,
    pub node_type: Type,
}
