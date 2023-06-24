use std::{fmt::Display, sync::mpsc::{Receiver, Sender, channel}, time::Duration, iter::Successors};

use crate::job::Job;

pub struct DebugNode<I, O> where I: Sized, O: Sized {
    pub name: String,
    pub connectors: Vec<Sender<I>>,
    pub input: Vec<Receiver<I>>,
    pub output: Vec<Sender<O>>,
}

impl<I, O> DebugNode<I, O> {
    pub fn new(name: &str) -> Self {
        let (in1, out1) = channel();
        Self { name: name.into(), connectors: vec![in1], input: vec![out1], output: vec![] }
    }
}

impl<I, O> Job<I, O> for DebugNode<I, O> where I: Display + Clone, O: Clone {
    fn handle(&mut self) {
        print!("{} node prints: ", self.name);
        self.input.iter().for_each(|c| {
            match c.recv_timeout(Duration::from_millis(10)) {
                Err(_) => println!("Nothing"),
                Ok(x) => println!("{}", x)
            };
        });
    }

    fn input(&self) -> &Vec<Sender<I>> {
        &self.connectors
    }

    fn output(&self) -> &Vec<Sender<O>> {
        &self.output
    }

    fn connect(&mut self, successors: Vec<Sender<O>>) -> &Self {
        for succ in successors {
            let _ = &self.output.push(succ);
        }
        self
    }
}