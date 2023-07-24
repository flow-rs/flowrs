use std::{
    fmt,
    sync::{
        mpsc::{channel, Receiver, RecvError, SendError, Sender, TryRecvError},
        Arc, Mutex,
    },
};

#[derive(Debug)]
pub enum ConnectError<I> {
    SendErr(SendError<I>),
    RecvErr(RecvError),
    TryRecvErr(TryRecvError),
    ChanErr(ChannelError),
}

#[derive(Debug, Clone)]
pub struct ChannelError {
    index: usize,
    size: usize,
}

impl<I> From<SendError<I>> for ConnectError<I> {
    fn from(value: SendError<I>) -> Self {
        ConnectError::SendErr(value)
    }
}

impl<I> From<RecvError> for ConnectError<I> {
    fn from(value: RecvError) -> Self {
        ConnectError::RecvErr(value)
    }
}

impl<I> From<TryRecvError> for ConnectError<I> {
    fn from(value: TryRecvError) -> Self {
        ConnectError::TryRecvErr(value)
    }
}

impl<I> From<ChannelError> for ConnectError<I> {
    fn from(value: ChannelError) -> Self {
        ConnectError::ChanErr(value)
    }
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "This Node has not enough inputs. Attempted write on input {} while this node only has {} inputs.", self.index, self.size)
    }
}

#[derive(Clone)]
pub struct Edge<I> {
    sender: Sender<I>,
    receiver: Arc<Mutex<Receiver<I>>>,
}

impl<I> Edge<I> {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn send(&self, elem: I) -> Result<(), ConnectError<I>> {
        Ok(self.sender.send(elem)?)
    }

    pub fn next_elem(&self) -> Result<I, ConnectError<I>> {
        Ok(self.receiver.lock().unwrap().try_recv()?)
    }
}

pub type Input<I> = Edge<I>;

#[derive(Clone)]
pub struct Output<T>(Arc<Mutex<Option<Edge<T>>>>);

impl<O> Output<O> {
    pub fn new() -> Self {
        Self (Arc::new(Mutex::new(None)))
    }
    
    pub fn send(&mut self, elem: O) -> Result<(), ConnectError<O>> {
        self.0.lock().unwrap().as_mut().unwrap().send(elem)
    }
}

pub fn connect<I>(lhs: &mut Option<Edge<I>>, rhs: Edge<I>) {
    *lhs = Some(rhs)
}
