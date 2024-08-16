use async_trait::async_trait;
use std::fmt;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use super::{communication::Communicator, messages::MessageError};
use crate::comm::messages::Message;

#[derive(Debug)]
pub struct NetworkCommunicator {
    stream: BufReader<TcpStream>,
}

impl NetworkCommunicator {
    pub async fn new(addr: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let stream = TcpStream::connect(addr).await?;
        Ok(NetworkCommunicator {
            stream: BufReader::new(stream),
        })
    }
}

#[async_trait]
impl Communicator for NetworkCommunicator {
    async fn send(&mut self, message: Message) -> Result<(), Box<dyn std::error::Error>> {
        self.stream
            .get_mut()
            .write_all(message.to_string().as_bytes())
            .await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message, Box<dyn std::error::Error>> {
        let mut line = String::new();
        self.stream.read_line(&mut line).await?;
        match Message::from_str(&line) {
            Some(message) => Ok(message),
            None => Err(Box::new(MessageError::CouldNotParse(line))),
        }
    }
}

impl fmt::Display for NetworkCommunicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PartialEq for NetworkCommunicator {
    fn eq(&self, other: &Self) -> bool {
        match (
            self.stream.get_ref().peer_addr(),
            other.stream.get_ref().peer_addr(),
        ) {
            (Ok(addr1), Ok(addr2)) => addr1 == addr2,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{net::TcpListener, spawn, sync::oneshot};

    use super::*;

    async fn run_test_server(shutdown_rx: oneshot::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
        let _ = spawn(async move {
            tokio::select! {
                _ = async {
                    loop {
                        let (mut socket, _) = listener.accept().await.unwrap();
                        let (reader, mut writer) = socket.split();
                        let mut reader = BufReader::new(reader);
                        let mut buffer = String::new();
                        reader.read_line(&mut buffer).await.unwrap();
                        writer.write_all(buffer.as_bytes()).await.unwrap();
                        writer.flush().await.unwrap();
                    }
                } => {},
                _ = shutdown_rx => {
                    println!("Server is shutting down.");
                }
            }
        });
    }

    #[tokio::test]
    async fn test_display_trait() {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        run_test_server(shutdown_rx).await;

        let comm = NetworkCommunicator::new("127.0.0.1:8080")
            .await
            .expect("should construct");
        assert_eq!(comm.to_string(), format!("{}", comm));
        shutdown_tx.send(()).unwrap();
    }
}
