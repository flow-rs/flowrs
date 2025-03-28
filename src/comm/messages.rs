use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use flowrs_package::flow_package::package::Type;
use std::{error::Error, fmt, str::FromStr};

use crate::flow::flow_types::{NodeIOIndex, NodeId};
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
    OrchestratorRequestNodeConnection(NodeId, NodeId, RuntimeId, String, NodeIOIndex, NodeIOIndex),
    RequestPeerConnection(NodeId, NodeId, NodeIOIndex, NodeIOIndex, Type),
    AcceptPeerConnection(NodeId, NodeId, NodeIOIndex, NodeIOIndex, u16),
    RejectPeerConnection(NodeId, NodeId, NodeIOIndex, NodeIOIndex, String),
    AcknowledgeConnectionSetup(NodeId, NodeId, NodeIOIndex, NodeIOIndex),

    // P2P Connection - IP Resolution
    RequestNodeRuntimeIP(NodeId),
    RespondNodeRuntimeIP(NodeId, String),

    // Debug & Data Messages
    Debug(String),
    Data(DataWrapper<D>),
}

// Message Constants
pub const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
pub const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";
pub const SETUP_COMMUNICATION_PREFIX: &str = "[[MESSAGE]: SetupCommunication]>";
pub const SETUP_COMMUNICATION_COMM: &str = "[Communicator:[";
pub const SETUP_COMMUNICATION_TYPE: &str = "], Type:[";
pub const SETUP_COMMUNICATION_PORT_PREFIX: &str = "[[MESSAGE]: SetupCommunicationPort]>";
pub const ACKNOWLEDGE_CONNECTION: &str = "[[MESSAGE]: AcknowledgeConnection]";
pub const DEBUG: &str = "[[MESSAGE]: [DEBUG]>]";
pub const DATA: &str = "[[MESSAGE]: [DATA]>]";
pub const INITIALIZE_LOCAL_NODES: &str = "[[MESSAGE]: InitializeLocalNodes]";
pub const ACKNOWLEDGE_NODE_INITIALIZATION: &str = "[[MESSAGE]: AcknowledgeNodeInitialization]";
pub const ORCHESTRATOR_REQUEST_NODE_CONNECTION: &str =
    "[[MESSAGE]: OrchestratorRequestNodeConnection]>";
pub const REQUEST_PEER_CONNECTION: &str = "[[MESSAGE]: RequestPeerConnection]>";
pub const ACCEPT_PEER_CONNECTION: &str = "[[MESSAGE]: AcceptPeerConnection]>";
pub const REJECT_PEER_CONNECTION: &str = "[[MESSAGE]: RejectPeerConnection]>";
pub const ACKNOWLEDGE_CONNECTION_SETUP: &str = "[[MESSAGE]: AcknowledgeConnectionSetup]>";
pub const REQUEST_NODE_RUNTIME_IP: &str = "[[MESSAGE]: RequestNodeRuntimeIP]>";
pub const RESPOND_NODE_RUNTIME_IP: &str = "[[MESSAGE]: RespondNodeRuntimeIP]>";

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
    DEBUG,
    DATA,
    SETUP_COMMUNICATION_COMM,
    SETUP_COMMUNICATION_PORT_PREFIX,
    ACKNOWLEDGE_CONNECTION,
    REQUEST_NODE_RUNTIME_IP,
    RESPOND_NODE_RUNTIME_IP,
];

// Implement fmt::Debug
impl<D> fmt::Debug for Message<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Execution Commands
            Message::StartExecution => write!(f, "{}", START_EXECUTION),
            Message::StopExecution => write!(f, "{}", STOP_EXECUTION),

            // Connection Setup
            Message::SetupCommunication(comm) => write!(
                f,
                "{}{}{:?}{}{:?}]",
                SETUP_COMMUNICATION_PREFIX,
                SETUP_COMMUNICATION_COMM,
                comm.communicator,
                SETUP_COMMUNICATION_TYPE,
                comm.node_type
            ),
            Message::SetupCommunicationPort(port) => {
                write!(f, "{} {}", SETUP_COMMUNICATION_PORT_PREFIX, port)
            }
            Message::AcknowledgeConnection => write!(f, "{}", ACKNOWLEDGE_CONNECTION),

            // Node Initialization
            Message::InitializeLocalNodes => write!(f, "{}", INITIALIZE_LOCAL_NODES),
            Message::AcknowledgeNodeInitialization => {
                write!(f, "{}", ACKNOWLEDGE_NODE_INITIALIZATION)
            }

            // P2P Connection Coordination
            Message::OrchestratorRequestNodeConnection(n1, n2, r_id, r_ip, send_idx, recv_idx) => {
                write!(
                    f,
                    "{}{},{},{},{},{},{}",
                    ORCHESTRATOR_REQUEST_NODE_CONNECTION, n1, n2, r_id, r_ip, send_idx, recv_idx,
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
                    serde_json::to_string(dtype).unwrap()
                )
            }
            Message::AcceptPeerConnection(n1, n2, send_idx, recv_idx, port) => {
                write!(
                    f,
                    "{}{},{},{},{},{}",
                    ACCEPT_PEER_CONNECTION, n1, n2, send_idx, recv_idx, port
                )
            }
            Message::RejectPeerConnection(n1, n2, send_idx, recv_idx, reason) => {
                write!(
                    f,
                    "{}{},{},{},{},{}",
                    REJECT_PEER_CONNECTION, n1, n2, send_idx, recv_idx, reason
                )
            }
            Message::AcknowledgeConnectionSetup(n1, n2, send_idx, recv_idx) => {
                write!(
                    f,
                    "{}{},{},{},{}",
                    ACKNOWLEDGE_CONNECTION_SETUP, n1, n2, send_idx, recv_idx
                )
            }
            Message::RequestNodeRuntimeIP(node_id) => {
                write!(f, "{}{}", REQUEST_NODE_RUNTIME_IP, node_id)
            }
            Message::RespondNodeRuntimeIP(node_id, ip) => {
                write!(f, "{}{},{}", RESPOND_NODE_RUNTIME_IP, node_id, ip)
            }

            // Debugging & Data Transfer
            Message::Debug(msg) => write!(f, "[[MESSAGE]: [DEBUG]>]{}", msg),
            Message::Data(data) => write!(f, "[[MESSAGE]: [DATA]>]{:?}", data),
        }
    }
}

// Implement fmt::Display
impl<D> fmt::Display for Message<D>
where
    //D: Clone,
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
    //D: Clone,
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
            // Execution Commands
            Some(START_EXECUTION) => Some(Self::StartExecution),
            Some(STOP_EXECUTION) => Some(Self::StopExecution),

            // Connection Setup
            Some(SETUP_COMMUNICATION_PREFIX) => {
                let comm_start = s.find(SETUP_COMMUNICATION_COMM)? + SETUP_COMMUNICATION_COMM.len();
                let comm_end = s.find(SETUP_COMMUNICATION_TYPE)?;
                let comm_string = s[comm_start..comm_end].to_string();
                let communicator_option = NodeCommunicator::from_str(&comm_string);
                if communicator_option.is_none() {
                    return None;
                }
                let communicator = communicator_option.unwrap();
                let node_type_start = comm_end + SETUP_COMMUNICATION_TYPE.len();
                let node_type_end = s.rfind(']')?;
                let node_type_string = s[node_type_start..node_type_end].to_string();
                println!("{}", node_type_string);
                let node_type: Type =
                    serde_json::from_str(&node_type_string).expect("should deserialize");
                Some(Self::SetupCommunication(CommWrapper {
                    communicator: communicator,
                    node_type: node_type,
                }))
            }
            Some(SETUP_COMMUNICATION_PORT_PREFIX) => {
                let port_string = s
                    .replacen(SETUP_COMMUNICATION_PORT_PREFIX, "", 1)
                    .trim()
                    .to_string();
                println!("[DEBUG] Extracted Port String: '{}'", port_string.clone());
                let port = port_string.parse::<u16>().ok()?;
                Some(Self::SetupCommunicationPort(port))
            }
            Some(ACKNOWLEDGE_CONNECTION) => Some(Self::AcknowledgeConnection),

            // Node Initialization
            Some(INITIALIZE_LOCAL_NODES) => Some(Self::InitializeLocalNodes),
            Some(ACKNOWLEDGE_NODE_INITIALIZATION) => Some(Self::AcknowledgeNodeInitialization),

            // P2P Connection Coordination
            Some(ORCHESTRATOR_REQUEST_NODE_CONNECTION) => {
                let stripped_msg = s.replacen(ORCHESTRATOR_REQUEST_NODE_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();

                if parts.len() != 6 {
                    println!("[ERROR] Invalid number of parts for OrchestratorRequestNodeConnection: {:?}", parts);
                    return None;
                }

                let sender_id = match parts[0].trim().parse() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("[ERROR] Failed to parse sender_id: '{}'", parts[0].trim());
                        return None;
                    }
                };

                let receiver_id = match parts[1].trim().parse() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("[ERROR] Failed to parse receiver_id: '{}'", parts[1].trim());
                        return None;
                    }
                };

                let runtime_id = match parts[2].trim().parse() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("[ERROR] Failed to parse runtime_id: '{}'", parts[2].trim());
                        return None;
                    }
                };

                let runtime_ip = parts[3].trim().to_string();

                let send_idx = match parts[4].parse() {
                    Ok(val) => val,
                    Err(_) => {
                        println!(
                            "[ERROR] Failed to parse sender_out_idx: {}",
                            parts[4].trim()
                        );
                        return None;
                    }
                };

                let recv_idx = match parts[5].trim().parse() {
                    Ok(val) => val,
                    Err(_) => {
                        println!("[ERROR] Failed to parse recv_in_idx: {}", parts[5].trim());
                        return None;
                    }
                };

                println!(
        "[DEBUG] Parsed OrchestratorRequestNodeConnection: sender_id={}, receiver_id={}, runtime_id={}, runtime_ip={}, send_idx={}, recv_idx={}",
        sender_id, receiver_id, runtime_id, runtime_ip, send_idx, recv_idx
    );

                Some(Self::OrchestratorRequestNodeConnection(
                    sender_id,
                    receiver_id,
                    runtime_id,
                    runtime_ip,
                    send_idx,
                    recv_idx,
                ))
            }
            Some(REQUEST_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(REQUEST_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.splitn(5, ',').collect();
                if parts.len() == 5 {
                    let dtype_str = parts[4].trim();
                    let dtype: Type = match serde_json::from_str(dtype_str) {
                        Ok(parsed) => parsed,
                        Err(_) => return None,
                    };

                    return Some(Self::RequestPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                        parts[3].parse().ok()?,
                        dtype,
                    ));
                }
                None
            }
            Some(ACCEPT_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(ACCEPT_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 5 {
                    Some(Self::AcceptPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                        parts[3].parse().ok()?,
                        parts[4].parse().ok()?,
                    ))
                } else {
                    None
                }
            }
            Some(REJECT_PEER_CONNECTION) => {
                let stripped_msg = s.replacen(REJECT_PEER_CONNECTION, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 5 {
                    Some(Self::RejectPeerConnection(
                        parts[0].parse().ok()?,
                        parts[1].parse().ok()?,
                        parts[2].parse().ok()?,
                        parts[3].parse().ok()?,
                        parts[4].to_string(),
                    ))
                } else {
                    None
                }
            }
            Some(ACKNOWLEDGE_CONNECTION_SETUP) => {
                let stripped_msg = s.replacen(ACKNOWLEDGE_CONNECTION_SETUP, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').map(|part| part.trim()).collect();
                println!("[DEBUG] AcknowledgeConnectionSetup parts: {:?}", parts);

                if parts.len() == 4 {
                    let sender_id = parts[0].parse::<u128>();
                    if sender_id.is_err() {
                        println!(
                            "[ERROR] Failed to parse sender_id ({}): {:?}",
                            parts[0], sender_id
                        );
                    }

                    let receiver_id = parts[1].parse::<u128>();
                    if receiver_id.is_err() {
                        println!(
                            "[ERROR] Failed to parse receiver_id ({}): {:?}",
                            parts[1], receiver_id
                        );
                    }

                    let send_out_idx = parts[2].parse::<u128>();
                    if send_out_idx.is_err() {
                        println!(
                            "[ERROR] Failed to parse send_out_idx ({}): {:?}",
                            parts[2], send_out_idx
                        );
                    }

                    let recv_in_idx = parts[3].parse::<u128>();
                    if recv_in_idx.is_err() {
                        println!(
                            "[ERROR] Failed to parse recv_in_idx ({}): {:?}",
                            parts[3], recv_in_idx
                        );
                    }

                    // Check if any parsing failed
                    if sender_id.is_err()
                        || receiver_id.is_err()
                        || send_out_idx.is_err()
                        || recv_in_idx.is_err()
                    {
                        println!("[ERROR] One or more parsing errors occurred.");
                        return None;
                    }

                    Some(Self::AcknowledgeConnectionSetup(
                        sender_id.unwrap(),
                        receiver_id.unwrap(),
                        send_out_idx.unwrap(),
                        recv_in_idx.unwrap(),
                    ))
                } else {
                    println!(
                        "[ERROR] Invalid number of parts for AcknowledgeConnectionSetup: {:?}",
                        parts
                    );
                    None
                }
            }
            Some(REQUEST_NODE_RUNTIME_IP) => {
                let stripped_msg = s.replacen(REQUEST_NODE_RUNTIME_IP, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 1 {
                    Some(Self::RequestNodeRuntimeIP(parts[0].parse().ok()?))
                } else {
                    None
                }
            }
            Some(RESPOND_NODE_RUNTIME_IP) => {
                let stripped_msg = s.replacen(RESPOND_NODE_RUNTIME_IP, "", 1);
                let parts: Vec<&str> = stripped_msg.split(',').collect();
                if parts.len() == 2 {
                    Some(Self::RespondNodeRuntimeIP(
                        parts[0].parse().ok()?,
                        parts[1].to_string(),
                    ))
                } else {
                    None
                }
            }

            // Debugging & Data Transfer
            Some("[[MESSAGE]: [DEBUG]>]") => {
                Some(Self::Debug(s.replacen("[[MESSAGE]: [DEBUG]>]", "", 1)))
            }
            Some("[[MESSAGE]: [DATA]>]") => {
                DataWrapper::parse(s.replacen("[[MESSAGE]: [DATA]>]", "", 1))
                    .ok()
                    .map(Self::Data)
            }

            // Default case: unknown message
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comm::communication::CommWrapper;
    use crate::comm::thread_communicator::ThreadCommunicator;
    use crate::flow::flow_types::NodeId;
    use crate::sched::scheduling_config::RuntimeId;
    use flowrs_package::flow_package::package::Type;

    /// Example Type for Serialization Testing
    const TYPE_JSON: &str = r#"
    {
        "inputs": null,
        "outputs": null,
        "type_parameters": [{"name": "U", "where": []}, {"name": "T", "where": []}],
        "constructors":{
            "New":{"NewWithObserver": {}},
            "FromCode":{"FromCode":{"code_template": "let {{fully_qualified_name}}:{{type_parameter_U}} = 5;"}}
        }
    }
    "#;

    #[tokio::test]
    async fn test_from_str() {
        if let Some(start_msg) = Message::<u32>::from_str(START_EXECUTION) {
            assert_eq!(start_msg, Message::StartExecution)
        }
        if let Some(stop_msg) = Message::<u32>::from_str(STOP_EXECUTION) {
            assert_eq!(stop_msg, Message::StopExecution)
        }
        if let Some(debug_msg) = Message::<u32>::from_str(DEBUG) {
            assert_eq!(debug_msg, Message::Debug("".to_string()))
        }
        // if let Some(data_msg) = Message::from_str(DATA) {
        //     assert_eq!(data_msg, Message::Data(DataWrapper {}))
        // }
        let comm = NodeCommunicator::ThreadComm(
            ThreadCommunicator::<u32>::new().expect("should construct"),
        );
        let node_type = serde_json::from_str(TYPE_JSON).expect("should deserialize");
        let comm_wrapper = CommWrapper {
            communicator: comm,
            node_type: node_type,
        };
        let format_str = format!(
            "{}{}{}{}{}]",
            SETUP_COMMUNICATION_PREFIX,
            SETUP_COMMUNICATION_COMM,
            comm_wrapper.communicator,
            SETUP_COMMUNICATION_TYPE,
            serde_json::to_string(&(comm_wrapper.node_type)).expect("should serialize"),
        );
        if let Some(setup_communication_msg) = Message::<u32>::from_str(&format_str) {
            assert_eq!(
                setup_communication_msg,
                Message::SetupCommunication(comm_wrapper)
            );
        }
        if let Some(setup_communication_port_msg) =
            Message::<u32>::from_str(format!("{}5050", SETUP_COMMUNICATION_PORT_PREFIX).as_str())
        {
            assert_eq!(
                setup_communication_port_msg,
                Message::SetupCommunicationPort(5050)
            )
        }
        if let Some(acknowledge_connection_msg) = Message::<u32>::from_str(ACKNOWLEDGE_CONNECTION) {
            assert_eq!(acknowledge_connection_msg, Message::AcknowledgeConnection)
        }
    }

    #[test]
    fn test_p2p_connection_messages() {
        let n1: NodeId = 1;
        let n2: NodeId = 2;
        let r_id: RuntimeId = 3;
        let r_ip = "192.168.1.1".to_string();
        let sender_out_idx: u128 = 10; // Example output index
        let receiver_in_idx: u128 = 20; // Example input index
        let dtype: Type = serde_json::from_str(TYPE_JSON).unwrap();

        // Updated test with output and input indices
        let msg_str = format!(
            "{}{},{},{},{},{},{}",
            ORCHESTRATOR_REQUEST_NODE_CONNECTION,
            n1,
            n2,
            r_id,
            r_ip,
            sender_out_idx,
            receiver_in_idx
        );

        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::OrchestratorRequestNodeConnection(
                n1,
                n2,
                r_id,
                r_ip.clone(),
                sender_out_idx,  // Added field
                receiver_in_idx  // Added field
            ))
        );

        let msg_str = format!(
            "{}{},{},{},{},{}",
            REQUEST_PEER_CONNECTION,
            n1,
            n2,
            0,
            1,
            serde_json::to_string(&dtype).unwrap()
        );
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::RequestPeerConnection(n1, n2, 0, 1, dtype.clone()))
        );

        let msg_str = format!(
            "{}{},{},{},{},{}",
            ACCEPT_PEER_CONNECTION, n1, n2, sender_out_idx, receiver_in_idx, 5050
        );
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::AcceptPeerConnection(
                n1,
                n2,
                sender_out_idx,
                receiver_in_idx,
                5050
            ))
        );

        let msg_str = format!(
            "{}{},{},{},{},{}",
            REJECT_PEER_CONNECTION, n1, n2, sender_out_idx, receiver_in_idx, "Invalid Connection"
        );
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::RejectPeerConnection(
                n1,
                n2,
                sender_out_idx,
                receiver_in_idx,
                "Invalid Connection".to_string()
            ))
        );

        let msg_str = format!(
            "{}{},{},{},{}",
            ACKNOWLEDGE_CONNECTION_SETUP, n1, n2, sender_out_idx, receiver_in_idx
        );
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::AcknowledgeConnectionSetup(
                n1,
                n2,
                sender_out_idx,
                receiver_in_idx,
            ))
        );

        let node_id: NodeId = 5;
        let node_ip = "192.168.1.10".to_string();

        // Test RequestNodeRuntimeIP
        let msg_str = format!("{}{}", REQUEST_NODE_RUNTIME_IP, node_id);
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::RequestNodeRuntimeIP(node_id))
        );

        // Test RespondNodeRuntimeIP
        let msg_str = format!("{}{},{}", RESPOND_NODE_RUNTIME_IP, node_id, node_ip);
        assert_eq!(
            Message::<String>::from_str(&msg_str),
            Some(Message::RespondNodeRuntimeIP(node_id, node_ip.clone()))
        );
    }
}
