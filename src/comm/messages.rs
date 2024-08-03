use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use std::fmt;

use super::{communication::CommWrapper, data::DataWrapper};

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
pub enum Message {
    StartExecution,
    StopExecution,
    SetupCommunication(CommWrapper),
    Debug(String),
    Data(DataWrapper),
}

pub const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
pub const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";
pub const SETUP_COMMUNICATION_PREFIX: &str = "[[MESSAGE]: SetupCommunication]";
pub const SETUP_COMMUNICATION_COMM: &str = "[Communicator:[";
pub const SETUP_COMMUNICATION_TYPE: &str = "], Type:[";
pub const DEBUG: &str = "[[MESSAGE]: [DEBUG]>]";
pub const DATA: &str = "[[MESSAGE]: [DATA]>]";

const PATTERNS: &[&str] = &[START_EXECUTION, STOP_EXECUTION, DEBUG];

impl fmt::Debug for Message {
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
        }
    }
}

impl fmt::Display for Message {
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
                comm.node_type
            ),
        }
    }
}

impl Message {
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
            Some(DATA) => Some(Self::Data(DataWrapper::parse(s.replacen(DEBUG, "", 1)))),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_from_str() {
        if let Some(start_msg) = Message::from_str(START_EXECUTION) {
            assert_eq!(start_msg, Message::StartExecution)
        }
        if let Some(stop_msg) = Message::from_str(STOP_EXECUTION) {
            assert_eq!(stop_msg, Message::StopExecution)
        }
        if let Some(debug_msg) = Message::from_str(DEBUG) {
            assert_eq!(debug_msg, Message::Debug("".to_string()))
        }
        if let Some(data_msg) = Message::from_str(DATA) {
            assert_eq!(data_msg, Message::Data(DataWrapper {}))
        }
    }
}
