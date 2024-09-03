use async_trait::async_trait;
use std::{fmt, pin::Pin, str::FromStr};
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
impl<D> Communicator<D> for NetworkCommunicator
where
    D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    async fn send(&mut self, message: Message<D>) -> Result<(), Box<dyn std::error::Error>> {
        self.stream
            .get_mut()
            .write_all(message.to_string().as_bytes())
            .await?;
        self.stream.get_mut().flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        self.stream.read_line(&mut line).await?;
        match Message::from_str(&line) {
            Some(message) => Ok(message),
            None => Err(Box::new(MessageError::CouldNotParse(line))),
        }
    }

    async fn try_receive(&mut self) -> Result<Option<Message<D>>, Box<dyn std::error::Error>> {
        let mut line = String::new();
        let reader = Pin::new(&mut self.stream);
        //buffer of size = 1 is enough to peak if a new message is available
        let mut buffer = [0, 1];
        // Using peek() to find available data. This is blocking but does not consume data
        // and has little overhead
        if let Ok(available) = reader.get_ref().peek(&mut buffer).await {
            if available > 0 {
                self.stream.read_line(&mut line).await?;
                return Ok(Message::from_str(&line));
            }
        }
        Ok(None)
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
    use std::{error::Error, net::SocketAddr};

    use tokio::{net::TcpListener, spawn, sync::oneshot};

    use super::*;

    struct TestServerShutdownGuard {
        shutdown_tx: Option<oneshot::Sender<()>>,
    }

    impl Drop for TestServerShutdownGuard {
        fn drop(&mut self) {
            if let Some(shutdown_tx) = self.shutdown_tx.take() {
                let _ = shutdown_tx.send(());
            }
        }
    }

    async fn run_test_server() -> (SocketAddr, TestServerShutdownGuard) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let guard = TestServerShutdownGuard {
            shutdown_tx: Some(shutdown_tx),
        };

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

        (addr, guard)
    }

    #[tokio::test]
    async fn test_display_trait() {
        let (addr, _guard) = run_test_server().await;

        let comm = NetworkCommunicator::new(&addr.to_string())
            .await
            .expect("should construct");
        //tests Display trait
        assert_eq!(comm.to_string(), format!("{}", comm));
    }

    #[tokio::test]
    async fn test_send_receive() {
        let (addr, _guard) = run_test_server().await;

        let mut comm = NetworkCommunicator::new(&addr.to_string())
            .await
            .expect("should construct");

        //Send something
        let test_data: String = "Test Data\n".to_string();
        let msg = Message::<String>::Debug(test_data.clone());
        let send_res = comm.send(msg).await;

        assert!(send_res.is_ok());

        //Try to receive
        let recv_res: Result<Message<String>, Box<dyn Error>> = comm.receive().await;
        assert!(recv_res.is_ok());
        let received_msg: Message<String> = recv_res.unwrap();
        match received_msg {
            Message::Debug(data) => assert_eq!(data, test_data),
            _ => panic!("Received message is not of type Debug"),
        }
    }
}
