use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use flowrs_package::flow_package::package::Type;
use std::{fmt, str::FromStr};

use super::{
    communication::{CommWrapper, NodeCommunicator},
    data::DataWrapper,
};

#[derive(Debug)]
pub enum MessageError {
    CouldNotParse(String),
    Empty,
}

impl std::error::Error for MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(PartialEq)]
pub enum Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    StartExecution,
    StopExecution,
    SetupCommunication(CommWrapper<D>),
    SetupCommunicationPort(u16),
    AcknowledgeConnection,
    Debug(String),
    Data(DataWrapper<D>),
}

pub const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
pub const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";
pub const SETUP_COMMUNICATION_PREFIX: &str = "[[MESSAGE]: SetupCommunication]>";
pub const SETUP_COMMUNICATION_COMM: &str = "[Communicator:[";
pub const SETUP_COMMUNICATION_TYPE: &str = "], Type:[";
pub const SETUP_COMMUNICATION_PORT_PREFIX: &str = "[[MESSAGE]: SetupCommunicationPort]>";
pub const ACKNOWLEDGE_CONNECTION: &str = "[[MESSAGE]: AcknowledgeConnection]";
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
    SETUP_COMMUNICATION_PORT_PREFIX,
    ACKNOWLEDGE_CONNECTION,
];

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
            Message::Debug(msg) => write!(f, "{}{}", DEBUG, msg),
            Message::Data(data) => write!(f, "{}{:?}", DATA, data),
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
        }
    }
}

impl<D> fmt::Display for Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::StartExecution => write!(f, "{}", START_EXECUTION),
            Message::StopExecution => write!(f, "{}", STOP_EXECUTION),
            Message::Debug(msg) => write!(f, "{}{}", DEBUG, msg),
            Message::Data(data) => write!(f, "{}{}", DATA, data),
            Message::SetupCommunication(comm) => write!(
                f,
                "{}{}{}{}{}]",
                SETUP_COMMUNICATION_PREFIX,
                SETUP_COMMUNICATION_COMM,
                comm.communicator,
                SETUP_COMMUNICATION_TYPE,
                serde_json::to_string(&(comm.node_type)).expect("should serialize"),
            ),
            Message::SetupCommunicationPort(port) => {
                write!(f, "{} {}", SETUP_COMMUNICATION_PORT_PREFIX, port)
            }
            Message::AcknowledgeConnection => write!(f, "{}", ACKNOWLEDGE_CONNECTION),
        }
    }
}

impl<D> Message<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    // use aho_corasick crate to match string prefix, see https://stackoverflow.com/a/64322185
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
            Some(DEBUG) => Some(Self::Debug(s.replacen(DEBUG, "", 1))),
            Some(DATA) => DataWrapper::parse(s.replacen(DATA, "", 1))
                .ok()
                .map(Self::Data),
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
                //let mut deserializer = Deserializer::from_str(&node_type_string);
                //let mut deserializer = serde_json::from_str(&node_type_string);
                //let node_type = Type::deserialize(&mut deserializer).expect("should deserialize");
                //let node_type = Type::deserialize(&mut Deserializer::from_str(&node_type_string))
                //    .expect("should deserialize");
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
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::comm::thread_communicator::ThreadCommunicator;

    use super::*;

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
}
