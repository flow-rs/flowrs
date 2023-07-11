use std::{
    fmt,
    sync::mpsc::{channel, Receiver, RecvError, SendError, Sender},
    vec,
};

use serde::Deserializer;

use crate::job::Connectable;

#[derive(Debug)]
pub enum ConnectError<I> {
    SendErr(SendError<I>),
    RecvErr(RecvError),
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

pub struct Connection<I, O> {
    connectors: Vec<Sender<I>>,
    pub state: Vec<Option<I>>,
    pub input: Vec<Receiver<I>>,
    input_size: usize,
    output: Vec<Sender<O>>,
}

impl<'de, I, O> serde::Deserialize<'de> for Connection<I, O> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let input_size = usize::deserialize(d)?;
        Ok(Connection::<I, O>::new(input_size))
    }
}

impl<I, O> Connection<I, O> {
    pub fn new(inputs: usize) -> Self {
        let mut connectors = vec![];
        let mut input = vec![];
        let mut state = vec![];
        for _ in 0..inputs {
            let (sender, receiver) = channel();
            connectors.push(sender);
            input.push(receiver);
            state.push(None);
        }
        Self {
            connectors,
            state,
            input,
            output: vec![],
            input_size: inputs,
        }
    }
}

impl<I, O> Connectable<I, O> for Connection<I, O>
where
    I: Clone,
    O: Clone,
{
    fn inputs(&self) -> &Vec<Sender<I>> {
        &self.connectors
    }

    fn output(&self) -> &Vec<Sender<O>> {
        &self.output
    }

    fn chain(&mut self, successors: Vec<Sender<O>>) {
        for succ in successors {
            let _ = &self.output.push(succ);
        }
    }

    fn send_out(&self, elem: O) {
        self.output().iter().for_each(|chan| {
            let _ = chan.send(elem.clone());
        });
    }

    fn send_at(&self, index: usize, value: I) -> Result<(), ConnectError<I>> {
        match self.inputs().get(index) {
            Some(chan) => Ok(chan.clone().send(value)?),
            None => Err(ConnectError::ChanErr(ChannelError {
                index,
                size: self.input_size,
            })),
        }
    }

    fn send(&self, value: I) -> Result<(), ConnectError<I>> {
        self.send_at(0, value)
    }

    fn input_at(&self, index: usize) -> Result<Sender<I>, ConnectError<I>> {
        match self.connectors.get(index).cloned() {
            Some(chan) => Ok(chan),
            None => Err(ConnectError::ChanErr(ChannelError {
                index,
                size: self.input_size,
            })),
        }
    }

    fn input(&self) -> Result<Sender<I>, ConnectError<I>> {
        self.input_at(0)
    }

    fn conn(&mut self) -> &mut Connection<I, O> {
        self
    }
}
