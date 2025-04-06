use async_trait::async_trait;
use std::{fmt, str::FromStr};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    time::{sleep, timeout, Duration},
};

use super::{communication::Communicator, messages::MessageError};
use crate::comm::messages::Message;

#[derive(Debug)]
pub struct NetworkCommunicator<T>
where
    T: fmt::Debug,
    T: FromStr,
    T: Send,
    T: 'static,
{
    stream: Option<BufReader<TcpStream>>,
    addr: Option<String>,
    port: Option<u16>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> NetworkCommunicator<T>
where
    T: fmt::Debug,
    T: FromStr,
    T: Send,
    T: 'static,
{
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(NetworkCommunicator {
            stream: None,
            addr: None,
            port: None,
            _phantom: std::marker::PhantomData,
        })
    }

    pub fn await_unwrap_sync() -> Self {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
        rt.block_on(Self::new())
            .expect("Failed to create NetworkCommunicator asynchronously")
    }

    pub fn dummy() -> Self {
        panic!("This dummy communicator should never be used. It's only here to support `mem::replace` for downcasting.");
    }

    pub async fn listen_single_message_on_fixed_port(
        port: u16,
    ) -> Result<Message<String>, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        let (mut socket, addr) = listener.accept().await?;
        println!("[NetworkCommunicator] Accepted R2R message from {}", addr);

        let mut line = String::new();
        let mut buffer = [0; 1];
        let mut buf_reader = BufReader::new(&mut socket);

        if buf_reader.get_ref().peek(&mut buffer).await? > 0 {
            buf_reader.read_line(&mut line).await?;
            if let Some(parsed_msg) = Message::from_str(&line) {
                return Ok(parsed_msg);
            } else {
                return Err("Failed to parse message".into());
            }
        }

        Err("No data available from peer".into())
    }
}

#[async_trait]
impl<D> Communicator<D> for NetworkCommunicator<D>
where
    //D: Clone,
    D: fmt::Debug,
    D: FromStr,
    D: Send,
    D: 'static,
{
    async fn send(
        &mut self,
        message: Message<D>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // if let Some(ref mut stream) = self.stream {
        //     // if the stream exists, write to it
        //     stream
        //         .get_mut()
        //         .write_all(message.to_string().as_bytes())
        //         .await?;
        //     stream.get_mut().flush().await?;
        //     Ok(())
        // } else {
        //     Err("Can not send, as the receiving stream was moved".into())
        // }

        // if let Some(ref mut stream) = self.stream {
        //     let msg_str = message.to_string();
        //     println!(
        //         "[DEBUG] Attempting to send message to {}:{} -> {:?}",
        //         self.addr.as_ref().unwrap_or(&"UNKNOWN".to_string()),
        //         self.port.unwrap_or(0),
        //         msg_str
        //     );

        //     let bytes = msg_str.as_bytes();
        //     println!("[DEBUG] Writing {} bytes...", bytes.len());

        //     match stream.get_mut().write_all(bytes).await {
        //         Ok(_) => println!("[DEBUG] write_all() completed successfully."),
        //         Err(e) => println!("[ERROR] write_all() failed: {}", e),
        //     }

        //     match stream.get_mut().flush().await {
        //         Ok(_) => println!("[DEBUG] flush() completed successfully."),
        //         Err(e) => println!("[ERROR] flush() failed: {}", e),
        //     }

        //     Ok(())
        // } else {
        //     Err("Cannot send, as the stream was moved".into())
        // }

        if self.stream.is_none() {
            return Err("Can not send, as the receiving stream was moved".into());
        }

        let msg_str = format!("{}\n", message.to_string()); // Ensure newline termination
        let mut attempts = 0;
        let max_retries = 5;
        let retry_delay = Duration::from_secs(1); // 1-second delay between retries

        while attempts < max_retries {
            if let Some(ref mut stream) = self.stream {
                match stream.get_mut().write_all(msg_str.as_bytes()).await {
                    Ok(_) => {
                        stream.get_mut().flush().await?;
                        println!(
                            "[DEBUG] Successfully sent message after {} attempt(s).",
                            attempts + 1
                        );
                        return Ok(()); // Message sent successfully
                    }
                    Err(e) => {
                        println!(
                            "[WARN] Failed to send message (attempt {}/{}): {}",
                            attempts + 1,
                            max_retries,
                            e
                        );
                        attempts += 1;
                        sleep(retry_delay).await; // Wait before retrying
                    }
                }
            } else {
                return Err("Can not send, as the receiving stream was moved".into());
            }
        }

        Err("Failed to send message after multiple retries.".into()) // Return error if all retries fail
    }

    async fn receive(&mut self) -> Result<Message<D>, Box<dyn std::error::Error + Send + Sync>> {
        if self.stream.is_none() {
            return Err("Cannot receive, as the receiving stream was moved".into());
        }
        let stream = self.stream.as_mut().unwrap();
        let mut line = String::new();
        let timeout_duration = Duration::from_secs(30);

        println!(
            "[DEBUG] Waiting to receive message on {}:{}",
            self.addr.as_ref().unwrap_or(&"UNKNOWN".to_string()),
            self.port.unwrap_or(0)
        );

        let result = timeout(timeout_duration, stream.read_line(&mut line)).await;

        match result {
            Ok(Ok(bytes_read)) => {
                println!("[DEBUG] Received {} bytes: '{}'", bytes_read, line.trim());

                if bytes_read == 0 {
                    return Err("Stream closed unexpectedly".into());
                }

                match Message::from_str(&line) {
                    Some(message) => {
                        println!("[DEBUG] Successfully parsed message: {:?}", message);
                        Ok(message)
                    }
                    None => {
                        println!("[ERROR] Failed to parse message: '{}'", line.trim());
                        Err(Box::new(MessageError::CouldNotParse(line)))
                    }
                }
            }
            Ok(Err(e)) => {
                println!("[ERROR] Read error: {}", e);
                Err(Box::new(e))
            }
            Err(_) => {
                println!(
                    "[ERROR] Timeout reached! No message received within {:?}.",
                    timeout_duration
                );
                Err("Timeout: No message received within the expected time".into())
            }
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
            println!("[DEBUG] Peeked {} bytes", available);
            if available > 0 {
                stream.read_line(&mut line).await?;
                println!("[DEBUG] Received line: {:?}", line);
                return Ok(Message::from_str(&line));
            }
        }
        Ok(None)
    }

    fn clone_send(&self) -> Self
    where
        Self: Sized,
    {
        NetworkCommunicator::<D> {
            stream: None,
            addr: self.addr.clone(),
            port: self.port.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    fn move_recv(&mut self) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
    where
        Self: Sized,
    {
        if let Some(stream) = self.stream.take() {
            Ok(NetworkCommunicator::<D> {
                stream: Some(stream),
                addr: self.addr.clone(),
                port: self.port.clone(),
                _phantom: std::marker::PhantomData,
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

impl<T> fmt::Display for NetworkCommunicator<T>
where
    T: fmt::Debug,
    T: FromStr,
    T: Send,
    T: 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl<T> PartialEq for NetworkCommunicator<T>
where
    T: fmt::Debug,
    T: FromStr,
    T: Send,
    T: 'static,
{
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

        let comm = NetworkCommunicator::<String>::new()
            .await
            .expect("should construct");
        //tests Display trait
        assert_eq!(comm.to_string(), format!("{}", comm));
    }

    #[tokio::test]
    async fn test_send_receive() {
        let (addr, _guard) = run_test_server().await;

        let mut comm = NetworkCommunicator::new().await.expect("should construct");
        let connect_res = comm
            .connect_send::<'_, '_>(Some(addr.ip().to_string()), Some(addr.port()))
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
