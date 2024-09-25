use std::{error::Error, fmt, str::FromStr};

use crate::comm::messages::Message;
use async_trait::async_trait;
use tokio::sync::{
    broadcast::error::RecvError,
    mpsc::{channel, Receiver, Sender},
};

use super::communication::Communicator;

const BUFFER_SIZE: usize = 10;

#[derive(Debug)]
pub struct ThreadCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    sender: Sender<Message<D>>,
    receiver: Option<Receiver<Message<D>>>,
}

impl<D> ThreadCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = channel(BUFFER_SIZE);

        Ok(ThreadCommunicator {
            sender: tx,
            receiver: Some(rx),
        })
    }
}

// impl<D> Clone for ThreadCommunicator<D>
// where
//     D: Clone,
//     D: fmt::Debug,
//     D: FromStr,
// {
//     fn clone(&self) -> Self {
//         Self {
//             sender: self.sender.clone(),
//             receiver: self.receiver.clone(),
//         }
//     }
// }

impl<D> fmt::Display for ThreadCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<D> PartialEq for ThreadCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
{
    // two ThreadCommunicators are equal when the sender and receiver objects are identical
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&(self.receiver), &(other.receiver))
            && std::ptr::eq(&(self.sender), &(other.sender))
    }
}

#[async_trait]
impl<D> Communicator<D> for ThreadCommunicator<D>
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    async fn send(&mut self, message: Message<D>) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .send(message)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error>> {
        // temporarily take the receiver from self
        let mut receiver = self
            .receiver
            .take()
            .expect("This communicator can only send but not receive");
        // use the receiver
        let result = receiver
            .recv()
            .await
            .ok_or_else(|| Box::new(RecvError::Closed) as Box<dyn Error>);
        //put back the receiver into self
        self.receiver = Some(receiver);
        result
    }

    async fn try_receive(&mut self) -> Result<Option<Message<D>>, Box<dyn std::error::Error>> {
        // temporarily take the receiver from self
        let mut receiver = self
            .receiver
            .take()
            .expect("This communicator can only send but not receive");
        // use the receiver
        let result = receiver
            .try_recv()
            .map_err(|e| Box::new(e) as Box<dyn Error>)
            .map(|res| Some(res));
        // put back the reveicer into self
        self.receiver = Some(receiver);
        result
    }

    fn clone_send(&self) -> Self
    where
        Self: Sized,
    {
        ThreadCommunicator {
            sender: self.sender.clone(),
            receiver: None,
        }
    }
    fn move_recv(&mut self) -> Result<Self, Box<dyn std::error::Error>>
    where
        Self: Sized,
    {
        if let Some(receiver) = self.receiver.take() {
            Ok(ThreadCommunicator {
                sender: self.sender.clone(),
                receiver: Some(receiver),
            })
        } else {
            Err("Receiver has already been moved".into())
        }
    }

    async fn connect_send(
        &mut self,
        _addr: Option<String>,
        _port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    async fn connect_recv(
        &mut self,
        _addr: Option<String>,
        _port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
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

    #[tokio::test]
    async fn test_empty_receive() {
        let mut communicator = ThreadCommunicator::<u32>::new().unwrap();
        let res = communicator.try_receive().await;
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
        let mut communicator = ThreadCommunicator::<u32>::new().unwrap();
        let send_res = communicator.send(Message::StartExecution).await;
        assert!(send_res.is_ok(), "Send should be successful");
        let recv_res = communicator.receive().await;
        assert!(recv_res.is_ok(), "Receive should be successful");
        let recv_msg = recv_res.unwrap();
        assert!(matches!(recv_msg, Message::StartExecution));
    }

    #[tokio::test]
    async fn test_threaded_comm() {
        let communicator = Arc::new(Mutex::new(ThreadCommunicator::<String>::new().unwrap()));

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
                        com.try_receive().await
                    };

                    if let Ok(message) = res {
                        match message {
                            Some(Message::StartExecution) => {
                                msg_1_counter += 1;
                                println!("received msg1");
                                if msg_1_counter + msg_2_counter == 6 {
                                    break;
                                }
                            }
                            Some(Message::StopExecution) => {
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
