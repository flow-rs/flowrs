pub trait Communicator {
    fn send(&self, message: Message) -> Result<Message, Box<dyn std::error::Error>>;
    fn receive(&self) -> Result<Message, Box<dyn std::error::Error>>;
}
