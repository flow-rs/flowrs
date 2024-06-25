use crate::comm::messages::Message;
use async_trait::async_trait;
use tokio::sync::mpsc::{
    channel,
    error::{SendError, TryRecvError},
    Receiver, Sender,
};

use super::communication::Communicator;

const BUFFER_SIZE: usize = 10;

pub struct ThreadCommunicator {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl ThreadCommunicator {
    pub fn new() -> Self {
        let (tx, rx) = channel(BUFFER_SIZE);

        ThreadCommunicator {
            sender: tx,
            receiver: rx,
        }
    }
}

#[async_trait]
impl Communicator for ThreadCommunicator {
    async fn send(&self, message: Message) -> Result<(), Box<SendError<Message>>> {
        self.sender.send(message).await.map_err(|e| e.into())
    }

    fn receive(&mut self) -> Result<Message, Box<TryRecvError>> {
        self.receiver.try_recv().map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {

    use std::sync::{Arc, Mutex};
    use std::thread::{sleep, spawn};
    use std::time::Duration;
    use std::time::Instant;

    use tokio::runtime::Runtime;

    use super::*;
    //use std::assert_matches::assert_matches;

    #[tokio::test]
    async fn test_empty_receive() {
        let mut communicator = ThreadCommunicator::new();
        let res = communicator.receive();
        assert!(
            res.is_err(),
            "receive() should return an error when the channel is empty."
        );
        assert!(
            matches!(*res.unwrap_err(), TryRecvError::Empty),
            "receive() should return TryRecvError::Empty when the achannel is empty."
        );
        //assert_matches!(*res.unwrap_err(), TryRecvError::Empty)
    }

    #[tokio::test]
    async fn test_send() {
        let mut communicator = ThreadCommunicator::new();
        let send_res = communicator.send(Message::StartExecution).await;
        assert!(send_res.is_ok(), "Send should be successfull");
        let recv_res = communicator.receive();
        assert!(recv_res.is_ok(), "Receive should be successful");
        let recv_msg = recv_res.unwrap();
        assert!(matches!(recv_msg, Message::StartExecution));
    }

    #[tokio::test]
    async fn test_threaded_comm() {
        let communicator = Arc::new(Mutex::new(ThreadCommunicator::new()));

        let communicator_clone_1 = Arc::clone(&communicator);
        let communicator_clone_2 = Arc::clone(&communicator);
        let communicator_clone_3 = Arc::clone(&communicator);

        let sender_1_fn = move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                {
                    let com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the first time");
                }
                sleep(Duration::from_millis(10));

                {
                    let com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the second time");
                }
                sleep(Duration::from_millis(10));

                {
                    let com = communicator_clone_1.lock().unwrap();
                    let _ = com.send(Message::StartExecution).await;
                    println!("sending msg1 for the third time");
                }
            });
        };

        let sender_2_fn = move || {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                {
                    let com = communicator_clone_2.lock().unwrap();
                    let _ = com.send(Message::StopExecution).await;
                    println!("sending msg2 for the first time");
                }
                sleep(Duration::from_millis(10));

                {
                    let com = communicator_clone_2.lock().unwrap();
                    let _ = com.send(Message::StopExecution).await;
                    println!("sending msg2 for the second time");
                }
                sleep(Duration::from_millis(10));

                {
                    let com = communicator_clone_2.lock().unwrap();
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
                        com.receive()
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
