use crate::comm::messages::Message;
use async_trait::async_trait;
use futures::{pin_mut, FutureExt};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc::error::TryRecvError;

use super::communication::Communicator;
use super::messages::MessageError;

pub struct ProcessCommunicator {
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl ProcessCommunicator {
    pub fn new(proc: Child) -> Result<Self, Box<dyn std::error::Error>> {
        //let proc = tokio::process::Command::new("echo").spawn()?;
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

#[async_trait]
impl Communicator for ProcessCommunicator {
    async fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
        self.stdin
            .write_all(message.to_string().as_bytes())
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn receive(&mut self) -> Result<Message, Box<dyn Error>> {
        let mut stdout_reader = BufReader::new(&mut self.stdout).lines();
        let mut stderr_reader = BufReader::new(&mut self.stderr).lines();
        let stdout_future = stdout_reader.next_line().fuse();
        let stderr_future = stderr_reader.next_line().fuse();

        // Pin futures to specific memory address
        // see https://users.rust-lang.org/t/why-is-pin-mut-needed-for-iteration-of-async-stream/51107
        pin_mut!(stdout_future);
        pin_mut!(stderr_future);

        let line: Option<String> = tokio::select! {
            line = stdout_future => {line.unwrap_or(None)}
            line = stderr_future => {line.unwrap_or(None)}
        };

        match line {
            Some(l) => {
                let msg = Message::from_str(&l);
                match msg {
                    Some(m) => Ok(m),
                    None => Err(Box::new(MessageError::CouldNotParse)),
                }
            }
            None => Err(Box::new(TryRecvError::Empty)),
        }
    }
}
