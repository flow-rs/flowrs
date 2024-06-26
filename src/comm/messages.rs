use std::fmt;

pub enum Message {
    StartExecution,
    StopExecution,
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartExecution => write!(f, "[[MESSAGE]: StartExecution]"),
            Self::StopExecution => write!(f, "[[MESSAGE]: StopExecution]"),
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StartExecution => write!(f, "[[MESSAGE]: StartExecution]"),
            Self::StopExecution => write!(f, "[[MESSAGE]: StopExecution]"),
        }
    }
}
