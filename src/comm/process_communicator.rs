use crate::comm::messages::Message;
use async_trait::async_trait;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::{
    channel,
    error::{SendError, TryRecvError},
};

use super::communication::Communicator;

pub struct ProcessCommunicator {
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl ProcessCommunicator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let proc = tokio::process::Command::new("echo").spawn()?;
        let stdin = proc.stdin.expect("StdIn should be provided");
        let stdout = proc.stdout.expect("StdOut should be provided");
        let stderr = proc.stderr.expect("StdErr should be provided");
        Ok(ProcessCommunicator {
            stdin: stdin,
            stdout: stdout,
            stderr: stderr,
        })
    }
}
