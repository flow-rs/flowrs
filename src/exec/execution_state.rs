
use std::fmt;

#[derive(PartialEq, Clone, Copy)]
pub enum ExecutionState {
    Ready,
    Sleeping,
    Running,
}


impl fmt::Display for ExecutionState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExecutionState::Ready => write!(f, "Ready"),
            ExecutionState::Sleeping => write!(f, "Sleeping"),
            ExecutionState::Running => write!(f, "Running"),
        }
    }
}