use std::{fmt::Display, sync::mpsc::{Receiver, Sender, channel}};

use crate::{job::{Job, Connectable}, log};

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

impl<I, O> Job for DebugNode<I, O> where I: Display + Clone, O: Clone {
    fn handle(&mut self) {
        print!("{} node prints: ", self.name);
        self.input.iter().for_each(|c| {
            // Avoiding recv_timout since wasm can't access system time without JS bindings
            let msg = match c.try_recv() {
                Err(_) => format!("Nothing"),
                Ok(x) => format!("{}", x)
            };
            // Log to Rust and Browser console
            println!("{}", msg);
            log(msg.as_str());
        });
    }

    fn name(&self) -> &String {
        &self.name
    }
}

impl<I, O> Connectable<I, O> for DebugNode<I, O> where I: Display + Clone, O: Clone {
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