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
