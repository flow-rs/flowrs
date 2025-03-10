use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use flowrs_package::flow_package::package::Type;
use std::{error::Error, fmt, str::FromStr};

use crate::flow::flow_types::NodeId;
use crate::sched::scheduling_config::RuntimeId;

use super::{
    communication::{CommWrapper, NodeCommunicator},
    data::DataWrapper,
};

#[derive(Debug)]
pub enum MessageError {
    CouldNotParse(String),
    Empty,
}

impl Error for MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::CouldNotParse(s) => write!(f, "Could not parse message: {}", s),
            MessageError::Empty => write!(f, "Empty message received"),
        }
    }
}

// Message Enum
#[derive(PartialEq)]
pub enum Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    // Execution
    StartExecution,
    StopExecution,

    // Connection Setup
    SetupCommunication(CommWrapper<D>),
    SetupCommunicationPort(u16),
    AcknowledgeConnection,

    // Node Initialization
    InitializeLocalNodes,
    AcknowledgeNodeInitialization,

    // P2P Connection Coordination
    OrchestratorRequestNodeConnection(NodeId, NodeId, RuntimeId, String),
    RequestPeerConnection(NodeId, NodeId, u16, u16, Type),
    AcceptPeerConnection(NodeId, NodeId, u16),
    RejectPeerConnection(NodeId, NodeId, String),
    AcknowledgeConnectionSetup(NodeId, NodeId, u16),

    // Debug & Data Messages
    Debug(String),
    Data(DataWrapper<D>),
}

// Message Constants
pub const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
pub const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";
pub const SETUP_COMMUNICATION_PREFIX: &str = "[[MESSAGE]: SetupCommunication]>";
pub const SETUP_COMMUNICATION_PORT_PREFIX: &str = "[[MESSAGE]: SetupCommunicationPort]>";
pub const ACKNOWLEDGE_CONNECTION: &str = "[[MESSAGE]: AcknowledgeConnection]";

// New Constants
pub const INITIALIZE_LOCAL_NODES: &str = "[[MESSAGE]: InitializeLocalNodes]";
pub const ACKNOWLEDGE_NODE_INITIALIZATION: &str = "[[MESSAGE]: AcknowledgeNodeInitialization]";
pub const ORCHESTRATOR_REQUEST_NODE_CONNECTION: &str =
    "[[MESSAGE]: OrchestratorRequestNodeConnection]>";
pub const REQUEST_PEER_CONNECTION: &str = "[[MESSAGE]: RequestPeerConnection]>";
pub const ACCEPT_PEER_CONNECTION: &str = "[[MESSAGE]: AcceptPeerConnection]>";
pub const REJECT_PEER_CONNECTION: &str = "[[MESSAGE]: RejectPeerConnection]>";
pub const ACKNOWLEDGE_CONNECTION_SETUP: &str = "[[MESSAGE]: AcknowledgeConnectionSetup]>";

// Integrate into Pattern Matching for Aho-Corasick
const PATTERNS: &[&str] = &[
    START_EXECUTION,
    STOP_EXECUTION,
    INITIALIZE_LOCAL_NODES,
    ACKNOWLEDGE_NODE_INITIALIZATION,
    ORCHESTRATOR_REQUEST_NODE_CONNECTION,
    REQUEST_PEER_CONNECTION,
    ACCEPT_PEER_CONNECTION,
    REJECT_PEER_CONNECTION,
    ACKNOWLEDGE_CONNECTION_SETUP,
    SETUP_COMMUNICATION_PREFIX,
    SETUP_COMMUNICATION_PORT_PREFIX,
    ACKNOWLEDGE_CONNECTION,
];

// Implement fmt::Debug
impl<D> fmt::Debug for Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::StartExecution => write!(f, "{}", START_EXECUTION),
            Message::StopExecution => write!(f, "{}", STOP_EXECUTION),
            Message::InitializeLocalNodes => write!(f, "{}", INITIALIZE_LOCAL_NODES),
            Message::AcknowledgeNodeInitialization => {
                write!(f, "{}", ACKNOWLEDGE_NODE_INITIALIZATION)
            }

            Message::OrchestratorRequestNodeConnection(n1, n2, r_id, r_ip) => {
                write!(
                    f,
                    "{}{},{},{},{}",
                    ORCHESTRATOR_REQUEST_NODE_CONNECTION, n1, n2, r_id, r_ip
                )
            }
            Message::RequestPeerConnection(n1, n2, o_idx, i_idx, dtype) => {
                write!(
                    f,
                    "{}{},{},{},{},{}",
                    REQUEST_PEER_CONNECTION,
                    n1,
                    n2,
                    o_idx,
                    i_idx,
                    serde_json::to_string(dtype).unwrap() // Ensure `dtype` is serialized correctly
                )
            }
            Message::AcceptPeerConnection(n1, n2, recv_port) => {
                write!(f, "{}{},{},{}", ACCEPT_PEER_CONNECTION, n1, n2, recv_port)
            }
            Message::RejectPeerConnection(n1, n2, reason) => {
                write!(f, "{}{},{},{}", REJECT_PEER_CONNECTION, n1, n2, reason)
            }
            Message::AcknowledgeConnectionSetup(n1, n2, recv_port) => {
                write!(
                    f,
                    "{}{},{},{}",
                    ACKNOWLEDGE_CONNECTION_SETUP, n1, n2, recv_port
                )
            }
            _ => write!(f, "[[MESSAGE]: UNKNOWN]"),
        }
    }
}

// Implement fmt::Display
impl<D> fmt::Display for Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f) // Just reuse the Debug implementation
    }
}

// Parsing Logic using Aho-Corasick
impl<D> Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn aho_corasick_match<T: AsRef<[u8]>>(ac: &AhoCorasick, v: T) -> Option<&'static str> {
        ac.find(&v).map(|m| PATTERNS[m.pattern()])
    }

    pub fn from_str(s: &str) -> Option<Self> {
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostLongest)
            .build(PATTERNS)
            .unwrap();

        match Self::aho_corasick_match(&ac, s) {
            Some(START_EXECUTION) => Some(Self::StartExecution),
            Some(STOP_EXECUTION) => Some(Self::StopExecution),
            Some(INITIALIZE_LOCAL_NODES) => Some(Self::InitializeLocalNodes),
            Some(ACKNOWLEDGE_NODE_INITIALIZATION) => Some(Self::AcknowledgeNodeInitialization),

            Some(ORCHESTRATOR_REQUEST_NODE_CONNECTION) => {
                let stripped_msg = s.replacen(ORCHESTRATOR_REQUEST_NODE_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 4 {
                    return Some(Self::OrchestratorRequestNodeConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                        parts[3].to_string(),
                    ));
                } else {
                    return None;
                }
            }
            Some(REQUEST_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(REQUEST_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 5 {
                    return Some(Self::RequestPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                        parts[3].parse().ok()?,
                        serde_json::from_str(parts[4]).ok()?,
                    ));
                } else {
                    return None;
                }
            }
            Some(ACCEPT_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(ACCEPT_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 3 {
                    return Some(Self::AcceptPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                    ));
                } else {
                    return None;
                }
            }
            Some(REJECT_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(REJECT_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 3 {
                    return Some(Self::RejectPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].to_string(),
                    ));
                } else {
                    return None;
                }
            }
            Some(ACKNOWLEDGE_CONNECTION_SETUP) => {
                let stripped_msg = s.replacen(ACKNOWLEDGE_CONNECTION_SETUP, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 3 {
                    return Some(Self::AcknowledgeConnectionSetup(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                    ));
                } else {
                    return None;
                }
            }
            _ => None,
        }
    }
}
