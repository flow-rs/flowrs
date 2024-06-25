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

    use std::thread::Thread;

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
}
