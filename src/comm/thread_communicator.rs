use crate::comm::messages::Message;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::communication::Communicator;

const BUFFER_SIZE: u8 = 10;

pub struct ThreadCommunicator {
    sender: Sender<Message>,
    receiver: Receiver<Message>,
}

impl ThreadCommunicator {
    pub fn new() -> Self {
        let (tx, rs) = channel(BUFFER_SIZE);

        ThreadCommunicator {
            sender: tx,
            receiver: rx,
        }
    }
}

impl Communicator for ThreadCommunicator {
    async fn send(&self, message: Message) -> Result<Message, Box<dyn std::error::Error>> {
        todo!()
    }

    async fn receive(&self) -> Result<Message, Box<dyn std::error::Error>> {
        todo!()
    }
}
