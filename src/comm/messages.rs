use std::fmt;

#[derive(Debug)]
pub enum MessageError {
    CouldNotParse,
}

impl std::error::Error for MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub enum Message {
    StartExecution,
    StopExecution,
}

const START_EXECUTION: &str = "[[MESSAGE]: StartExecution]";
const STOP_EXECUTION: &str = "[[MESSAGE]: StopExecution]";

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartExecution => write!(f, "{}", START_EXECUTION),
            Self::StopExecution => write!(f, "{}", STOP_EXECUTION),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartExecution => write!(f, "{}", START_EXECUTION),
            Self::StopExecution => write!(f, "{}", STOP_EXECUTION),
        }
    }
}

impl Message {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            START_EXECUTION => Some(Self::StartExecution),
            STOP_EXECUTION => Some(Self::StopExecution),
            _ => None,
        }
    }
}
