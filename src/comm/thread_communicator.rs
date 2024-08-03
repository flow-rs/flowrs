use std::{error::Error, fmt};

use crate::comm::messages::Message;
use async_trait::async_trait;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::communication::Communicator;

const BUFFER_SIZE: usize = 10;

#[derive(Debug)]
pub struct ThreadCommunicator {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl ThreadCommunicator {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = channel(BUFFER_SIZE);

        Ok(ThreadCommunicator {
            sender: tx,
            receiver: rx,
        })
    }
}

impl fmt::Display for ThreadCommunicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PartialEq for ThreadCommunicator {
    // two ThreadCommunicators are equal when the sender and receiver objects are identical
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&(self.receiver), &(other.receiver))
            && std::ptr::eq(&(self.sender), &(other.sender))
    }
}

#[async_trait]
impl Communicator for ThreadCommunicator {
    async fn send(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        Ok(self.sender.send(message).await.map_err(|e| Box::new(e))?)
    }

    async fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>> {
        Ok(self
            .receiver
            .try_recv()
            .map_err(|e| Box::new(e) as Box<dyn Error>)?)
    }
}

#[cfg(test)]
mod tests {

    use std::sync::{Arc, Mutex};
    use std::thread::{sleep, spawn};
    use std::time::Duration;
    use std::time::Instant;

    use tokio::runtime::Runtime;
    use tokio::sync::mpsc::error::TryRecvError;

    use super::*;
    //use std::assert_matches::assert_matches;

    #[tokio::test]
    async fn test_empty_receive() {
        let mut communicator = ThreadCommunicator::new().unwrap();
        let res = communicator.receive().await;
        assert!(
            res.is_err(),
            "receive() should return an error when the channel is empty."
        );

        let err = res.unwrap_err();
        assert!(matches!(
            err.downcast_ref::<TryRecvError>(),
            Some(TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn test_send() {
        let mut communicator = ThreadCommunicator::new().unwrap();
        let send_res = communicator.send(Message::StartExecution).await;
        assert!(send_res.is_ok(), "Send should be successful");
        let recv_res = communicator.receive().await;
        assert!(recv_res.is_ok(), "Receive should be successful");
        let recv_msg = recv_res.unwrap();
        assert!(matches!(recv_msg, Message::StartExecution));
    }

    #[tokio::test]
    async fn test_threaded_comm() {
        let communicator = Arc::new(Mutex::new(ThreadCommunicator::new().unwrap()));

        let communicator_clone_1 = Arc::clone(&communicator);
        let communicator_clone_2 = Arc::clone(&communicator);
        let communicator_clone_3 = Arc::clone(&communicator);

        let sender_1_fn = move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                {
                    let mut com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the first time");
                }
                sleep(Duration::from_millis(10));

                {
                    let mut com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the second time");
                }
                sleep(Duration::from_millis(10));

                {
                    let mut com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the third time");
                }
            });
        };

        let sender_2_fn = move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                {
                    let mut com = communicator_clone_2.lock().unwrap();
                    let _ = com.send(Message::StopExecution).await;
                    println!("sending msg2 for the first time");
                }
                sleep(Duration::from_millis(10));

                {
                    let mut com = communicator_clone_2.lock().unwrap();
                    let _ = com.send(Message::StopExecution).await;
                    println!("sending msg2 for the second time");
                }
                sleep(Duration::from_millis(10));

                {
                    let mut com = communicator_clone_2.lock().unwrap();
                    let _ = com.send(Message::StopExecution).await;
                    println!("sending msg2 for the third time");
                }
            });
        };

        let recv_fn = move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let start_time = Instant::now();
                let timeout = Duration::from_secs(3);
                let mut msg_1_counter: u8 = 0;
                let mut msg_2_counter: u8 = 0;
                while start_time.elapsed() < timeout {
                    let res = {
                        let mut com = communicator_clone_3.lock().unwrap();
                        com.receive().await
                    };

                    if let Ok(message) = res {
                        match message {
                            Message::StartExecution => {
                                msg_1_counter += 1;
                                println!("received msg1");
                                if msg_1_counter + msg_2_counter == 6 {
                                    break;
                                }
                            }
                            Message::StopExecution => {
                                msg_2_counter += 1;
                                println!("received msg2");
                                if msg_1_counter + msg_2_counter == 6 {
                                    break;
                                }
                            }
                            _ => {
                                assert!(false)
                            }
                        }
                    }
                }
                assert_eq!(msg_1_counter, 3);
                assert_eq!(msg_2_counter, 3);
            });
        };

        let sender_1_handle = spawn(sender_1_fn);
        let sender_2_handle = spawn(sender_2_fn);
        let recv_handle = spawn(recv_fn);

        sender_1_handle.join().unwrap();
        sender_2_handle.join().unwrap();
        recv_handle.join().unwrap();
    }
}
