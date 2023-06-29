use std::sync::mpsc::{channel, Receiver, Sender};

use crate::job::Connectable;

pub struct Connection<I, O> {
    connectors: Vec<Sender<I>>,
    pub input: Vec<Receiver<I>>,
    output: Vec<Sender<O>>,
}

impl<I, O> Connection<I, O> {
    pub fn new(inputs: usize) -> Self {
        let mut connectors = vec![];
        let mut input = vec![];
        for _ in 0..inputs {
            let (sender, receiver) = channel();
            connectors.push(sender);
            input.push(receiver);
        }
        Self {
            connectors,
            input,
            output: vec![],
        }
    }
}

impl<I, O> Connectable<I, O> for Connection<I, O> {
    fn input(&self) -> &Vec<Sender<I>> {
        &self.connectors
    }

    fn output(&self) -> &Vec<Sender<O>> {
        &self.output
    }

    fn chain(&mut self, successors: Vec<Sender<O>>) -> &Self {
        for succ in successors {
            let _ = &self.output.push(succ);
        }
        self
    }

    fn send_at(&self, index: usize, value: I) {
        let _ = &self.input()[index].send(value);
    }

    fn send(&self, value: I) {
        let _ = &self.input()[0].send(value);
    }
}
