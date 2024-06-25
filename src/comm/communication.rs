use crate::comm::messages::Message;
use async_trait::async_trait;
#[async_trait]
pub trait Communicator {
    async fn send(&self, message: Message) -> Result<Message, Box<dyn std::error::Error>>;
    async fn receive(&self) -> Result<Message, Box<dyn std::error::Error>>;
}
