use std::{ops::Add, sync::mpsc::{Sender, Receiver}, time::Duration};
use crate::job::Job;

pub struct AddNode<S, I, O> where I: Sized {
    pub state: Option<S>,
    pub neutral: I,
    pub name: String,
    pub input: Vec<Receiver<I>>,
    pub output: Vec<Sender<O>>,
    pub to_state: fn(I) -> S,
    pub from_state: fn(S) -> I,
}

impl<S, I, O> Job for AddNode<S, I, O> where I: Add<Output = O> + Clone, S: Clone, O: Clone {
    fn handle(&mut self) {
        self.state = match &self.state {
            None => match self.input.get(0) {
                None => None,
                Some(i) => {
                    let f = self.to_state;
                    match i.recv_timeout(Duration::from_millis(10)) {
                        Err(e) => return,
                        Ok(v) => Some(f(v)),
                    }
                }
            },
            Some(i) => {
                let v1 = self.input.get(1);
                let v = match v1 {
                    None => self.neutral.clone(),
                    Some(c) => match c.recv_timeout(Duration::from_millis(10)) {
                        Ok(w) => w,
                        // TODO: better error handling...
                        Err(_) => return
                    },
                };
                let f = self.from_state;
                // TODO: better error handling...
                match self.output.get(0) {
                    Some(c) => c.send(f(i.clone())+v.clone()),
                    None => return,
                };
                None
            },
        };
    }
}
