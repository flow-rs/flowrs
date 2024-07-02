use crate::comm::messages::Message;

use async_trait::async_trait;

#[async_trait]
pub trait Communicator {
    async fn send(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>>;
    async fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>>;
}
