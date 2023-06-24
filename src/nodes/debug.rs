use std::{fmt::Display, sync::mpsc::{Receiver, Sender}, time::Duration};

use crate::job::Job;

pub struct DebugNode<I> where I: Sized {
    pub name: String,
    pub input: Receiver<I>,
    pub output: Sender<I>,
}

impl<I> Job for DebugNode<I> where I: Display + Clone {
    fn handle(&mut self) {
        print!("{} node prints: ", self.name);
        match self.input.recv_timeout(Duration::from_millis(10)) {
            Err(_) => println!("Nothing"),
            Ok(x) => println!("{}", x)
        };
    }
}