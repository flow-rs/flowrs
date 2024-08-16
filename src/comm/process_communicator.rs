// use crate::comm::messages::Message;
// use async_trait::async_trait;
// use futures::{pin_mut, FutureExt};
// use std::error::Error;
// use std::fmt;
// use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
// use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
// use tokio::sync::mpsc::error::TryRecvError;

// use super::communication::Communicator;
// use super::messages::MessageError;

// #[derive(Debug)]
// pub struct ProcessCommunicator {
//     pub pid: u32,
//     stdin: ChildStdin,
//     stdout: ChildStdout,
//     stderr: ChildStderr,
// }

// impl ProcessCommunicator {
//     pub fn new(mut proc: Child) -> Result<Self, Box<dyn std::error::Error>> {
//         //let proc = tokio::process::Command::new("echo").spawn()?;
//         let pid = proc.id().expect("Process must be running");
//         let stdin = proc.stdin.take().unwrap(); //.expect("StdIn should be provided");
//         let stdout = proc.stdout.expect("StdOut should be provided");
//         let stderr = proc.stderr.expect("StdErr should be provided");
//         Ok(ProcessCommunicator {
//             pid: pid,
//             stdin: stdin,
//             stdout: stdout,
//             stderr: stderr,
//         })
//     }
// }

// impl fmt::Display for ProcessCommunicator {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         write!(f, "{:?}", self)
//     }
// }

// impl PartialEq for ProcessCommunicator {
//     // two ProcessCommunicators are equal when stdin, stdout and stderr are identical
//     fn eq(&self, other: &Self) -> bool {
//         std::ptr::eq(&self.stdin, &other.stdin)
//             && std::ptr::eq(&(self.stdout), &(other.stdout))
//             && std::ptr::eq(&(self.stderr), &(other.stderr))
//     }
// }

// #[async_trait]
// impl Communicator for ProcessCommunicator {
//     async fn send(&mut self, message: Message) -> Result<(), Box<dyn Error>> {
//         self.stdin
//             .write_all((message.to_string() + "\n").as_bytes())
//             .await
//             .map_err(|e| Box::new(e) as Box<dyn Error>)
//     }

//     async fn receive(&mut self) -> Result<Message, Box<dyn Error>> {
//         let mut stdout_reader = BufReader::new(&mut self.stdout).lines();
//         let mut stderr_reader = BufReader::new(&mut self.stderr).lines();
//         let stdout_future = stdout_reader.next_line().fuse();
//         let stderr_future = stderr_reader.next_line().fuse();

//         // Pin futures to specific memory address
//         // see https://users.rust-lang.org/t/why-is-pin-mut-needed-for-iteration-of-async-stream/51107
//         pin_mut!(stdout_future);
//         pin_mut!(stderr_future);

//         let line: Option<Result<String, std::io::Error>> = tokio::select! {
//             line = stdout_future => line.transpose(),  // Converts Result<Option<_>, _> to Option<Result<_>>
//             line = stderr_future => line.transpose(),
//         };

//         match line {
//             Some(Ok(l)) => {
//                 let msg = Message::from_str(&l);
//                 match msg {
//                     Some(m) => Ok(m),
//                     None => Err(Box::new(MessageError::CouldNotParse(l))),
//                 }
//             }
//             Some(Err(e)) => Err(Box::new(e)),
//             None => Err(Box::new(TryRecvError::Empty)),
//         }
//         // let line: Option<String> = tokio::select! {
//         //     line = stdout_future => {line.unwrap_or(Some("hello".to_string()))}
//         //     line = stderr_future => {line.unwrap_or(Some("hello".to_string()))}
//         // };

//         // //panic!("hi");
//         // match line {
//         //     Some(l) => {
//         //         let msg = Message::from_str(&l);
//         //         match msg {
//         //             Some(m) => Ok(m),
//         //             None => Err(Box::new(MessageError::CouldNotParse(l))),
//         //         }
//         //     }
//         //     None => Err(Box::new(TryRecvError::Empty)),
//         // }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use std::path::PathBuf;
//     use std::process::Stdio;

//     use super::*;
//     use tokio::process::Command;
//     use tokio::sync::mpsc::error::TryRecvError;

//     fn check_feature_enabled() {
//         if !cfg!(feature = "testing") {
//             panic!("The 'testing' feature must be enabled to run this test!");
//         }
//     }

//     //https://github.com/3tilley/rust-experiments/blob/337df8d52ddfb629f5bf0d9fe02beda0d408ff19/ipc/src/lib.rs#L37
//     fn executable_path(name: &str) -> PathBuf {
//         #[cfg(target_os = "windows")]
//         let exe = name.to_owned() + ".exe";
//         #[cfg(target_family = "unix")]
//         let exe = name.to_owned();

//         #[cfg(debug_assertions)]
//         let out = PathBuf::from("./target/debug/").join(exe);
//         #[cfg(not(debug_assertions))]
//         let out = PathBuf::from("./target/release/").join(exe);

//         out
//     }

//     #[tokio::test]
//     async fn test_empty_receive() {
//         let mut cmd = Command::new("cargo");
//         cmd.stdin(Stdio::piped());
//         cmd.stdout(Stdio::piped());
//         cmd.stderr(Stdio::piped());

//         let proc = cmd.spawn().expect("should spawn");
//         let mut communicator = ProcessCommunicator::new(proc).expect("should construct");
//         let res = communicator.receive().await;
//         assert!(
//             res.is_err(),
//             "receive() should return an error when the channel is empty."
//         );

//         let err = res.unwrap_err();
//         assert!(
//             matches!(
//                 err.downcast_ref::<TryRecvError>(),
//                 Some(TryRecvError::Empty)
//             ) || matches!(
//                 err.downcast_ref::<MessageError>(),
//                 Some(MessageError::CouldNotParse(_))
//             )
//         );
//     }

//     #[tokio::test]
//     async fn test_send() {
//         check_feature_enabled();
//         let mut cmd = {
//             let exe = executable_path("ipc_test");
//             println!("{:?}", exe);
//             let cmd = Command::new(exe);
//             cmd
//         };

//         cmd.stdin(Stdio::piped());
//         cmd.stdout(Stdio::piped());
//         cmd.stderr(Stdio::piped());

//         let mut communicator =
//             ProcessCommunicator::new(cmd.spawn().expect("should spawn")).expect("should construct");
//         let mut send_res = communicator.send(Message::StartExecution).await;
//         assert!(send_res.is_ok(), "Send should be successful");
//         let recv_res = communicator.receive().await;
//         assert!(recv_res.is_ok(), "Receive should be successful");
//         let msg = recv_res.unwrap();
//         assert_eq!(Message::StartExecution, msg);

//         //stop child Process
//         send_res = communicator.send(Message::StopExecution).await;
//         assert!(send_res.is_ok(), "Send should be successful");
//     }

//     #[tokio::test]
//     async fn test_process_comm() {
//         //check for testing feature
//         check_feature_enabled();

//         //set up communication partners
//         let mut cmd1 = {
//             let exe = executable_path("ipc_test_comm_one");
//             println!("{:?}", exe);
//             let cmd = Command::new(exe);
//             cmd
//         };
//         let mut cmd2 = {
//             let exe = executable_path("ipc_test_comm_two");
//             println!("{:?}", exe);
//             let cmd = Command::new(exe);
//             cmd
//         };
//         cmd1.stdin(Stdio::piped());
//         cmd1.stdout(Stdio::piped());
//         cmd1.stderr(Stdio::piped());
//         cmd2.stdin(Stdio::piped());
//         cmd2.stdout(Stdio::piped());
//         cmd2.stderr(Stdio::piped());
//         let mut communicator1 = ProcessCommunicator::new(cmd1.spawn().expect("should spawn"))
//             .expect("should construct");

//         let mut communicator2 = ProcessCommunicator::new(cmd1.spawn().expect("should spawn"))
//             .expect("should construct");
//     }
// }
