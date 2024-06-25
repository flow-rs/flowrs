use crate::comm::messages::Message;

use async_trait::async_trait;
use tokio::sync::mpsc::error::{SendError, TryRecvError};

#[async_trait]
pub trait Communicator {
    async fn send(&self, message: Message) -> Result<(), Box<SendError<Message>>>;
    fn receive(&mut self) -> Result<Message, Box<TryRecvError>>;
}
