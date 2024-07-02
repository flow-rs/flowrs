use crate::comm::messages::Message;

use async_trait::async_trait;

#[async_trait]
pub trait Communicator {
    async fn send(&self, message: Message) -> Result<(), Box<dyn std::error::Error>>;
    fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>>;
}
