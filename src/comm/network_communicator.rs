use async_trait::async_trait;
use std::{fmt, str::FromStr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use super::{communication::Communicator, messages::MessageError};
use crate::comm::messages::Message;

#[derive(Debug)]
pub struct NetworkCommunicator {
    stream: Option<BufReader<TcpStream>>,
    addr: Option<String>,
    port: Option<u16>,
}

impl NetworkCommunicator {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(NetworkCommunicator {
            stream: None,
            addr: None,
            port: None,
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
    async fn send(
        &mut self,
        message: Message<D>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref mut stream) = self.stream {
            // if the stream exists, write to it
            stream
                .get_mut()
                .write_all(message.to_string().as_bytes())
                .await?;
            stream.get_mut().flush().await?;
            Ok(())
        } else {
            Err("Can not send, as the receiving stream was moved".into())
        }
    }

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error + Send + Sync>> {
        if self.stream.is_none() {
            return Err("Can not receive, as the receiving stream was moved".into());
        }
        let stream = self.stream.as_mut().unwrap();
        let mut line = String::new();
        stream.read_line(&mut line).await?;
        match Message::from_str(&line) {
            Some(message) => Ok(message),
            None => Err(Box::new(MessageError::CouldNotParse(line))),
        }
    }

    async fn try_receive(
        &mut self,
    ) -> Result<Option<Message<D>>, Box<dyn std::error::Error + Send + Sync>> {
        if self.stream.is_none() {
            return Err("Can not receive, as the receiving stream was moved".into());
        }
        let stream = self.stream.as_mut().unwrap();
        let mut line = String::new();
        //buffer of size = 1 is enough to peak if a new message is available
        let mut buffer = [0, 1];
        // Using peek() to find available data. This is blocking but does not consume data
        // and has little overhead
        if let Ok(available) = stream.get_ref().peek(&mut buffer).await {
            if available > 0 {
                stream.read_line(&mut line).await?;
                return Ok(Message::from_str(&line));
            }
        }
        Ok(None)
    }

    fn clone_send(&self) -> Self
    where
        Self: Sized,
    {
        NetworkCommunicator {
            stream: None,
            addr: self.addr.clone(),
            port: self.port.clone(),
        }
    }
    fn move_recv(&mut self) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: Sized,
    {
        if let Some(stream) = self.stream.take() {
            Ok(NetworkCommunicator {
                stream: Some(stream),
                addr: self.addr.clone(),
                port: self.port.clone(),
            })
        } else {
            Err("Can not move stream as it is None".into())
        }
    }

    async fn connect_send(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if addr.is_none() || port.is_none() {
            return Err("IP Address and port must be given".into());
        }
        let net_addr = format!("{}:{}", addr.as_ref().unwrap(), port.unwrap());

        let stream = TcpStream::connect(net_addr).await?;
        self.stream = Some(BufReader::new(stream));
        self.addr = Some(addr.unwrap());
        self.port = Some(port.unwrap());
        Ok(())
    }

    async fn connect_recv(
        &mut self,
        addr: Option<String>,
        port: Option<u16>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if addr.is_none() || port.is_none() {
            return Err("IP Address and port must be given".into());
        }
        let listener = TcpListener::bind(("0.0.0.0", port.unwrap())).await?;
        let (stream, remote_addr) = listener.accept().await?;
        self.stream = Some(BufReader::new(stream));
        self.addr = Some(addr.clone().unwrap());
        self.port = Some(port.unwrap());
        if !(remote_addr.to_string() == addr.clone().unwrap()) {
            //strict check not working with docker's NAT resolution
            //return Err("Received connection from wrong IP".into());
            //display warning message instead
            println!(
            "[Node RT] WARN: Receiver accepted connection from {} while expected address was {}",
            remote_addr, addr.unwrap()
);
        }
        Ok(())
    }
}

impl fmt::Display for NetworkCommunicator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl PartialEq for NetworkCommunicator {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr && self.port == other.port
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
        //let (addr, _guard) = run_test_server().await;

        let comm = NetworkCommunicator::new().await.expect("should construct");
        //tests Display trait
        assert_eq!(comm.to_string(), format!("{}", comm));
    }

    #[tokio::test]
    async fn test_send_receive() {
        let (addr, _guard) = run_test_server().await;

        let mut comm = NetworkCommunicator::new().await.expect("should construct");
        let connect_res = <NetworkCommunicator as Communicator<String>>::connect_send::<'_, '_>(
            &mut comm,
            Some(addr.ip().to_string()),
            Some(addr.port()),
        )
        .await;
        assert!(connect_res.is_ok());

        //Send something
        let test_data: String = "Test Data\n".to_string();
        let msg = Message::<String>::Debug(test_data.clone());
        let send_res = comm.send(msg).await;

        assert!(send_res.is_ok());

        //Try to receive
        let recv_res: Result<Message<String>, Box<dyn Error + Send + Sync>> = comm.receive().await;
        assert!(recv_res.is_ok());
        let received_msg: Message<String> = recv_res.unwrap();
        match received_msg {
            Message::Debug(data) => assert_eq!(data, test_data),
            _ => panic!("Received message is not of type Debug"),
        }
    }
}
