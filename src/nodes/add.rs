use crate::job::Job;
use std::{
    ops::Add,
    sync::mpsc::{Receiver, Sender, channel},
    time::Duration, vec,
};

pub struct AddNode<S, I, O>
where
    I: Sized,
{
    pub state: Option<S>,
    neutral_ele: I,
    name: String,
    connectors: Vec<Sender<I>>,
    input: Vec<Receiver<I>>,
    output: Vec<Sender<O>>,
    to_state: fn(I) -> S,
    from_state: fn(S) -> I,
}

impl<S, I, O> AddNode<S, I, O> {
    pub fn new(name: &str, natural_ele: I, to_state: fn(I) -> S, from_state: fn(S) -> I) -> Self {
        let (in1, out1) = channel();
        let (in2, out2) = channel();
        Self {
            state: None,
            neutral_ele: natural_ele,
            name: name.into(),
            connectors: vec![in1, in2],
            input: vec![out1, out2],
            output: vec![],
            to_state: to_state,
            from_state: from_state,
        }
    }
}

impl<S, I, O> Job<I, O> for AddNode<S, I, O>
where
    I: Add<Output = O> + Clone,
    S: Clone,
    O: Clone,
{
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
                    None => self.neutral_ele.clone(),
                    Some(c) => match c.recv_timeout(Duration::from_millis(10)) {
                        Ok(w) => w,
                        // Nothing to do, skipping cycle
                        Err(_) => return,
                    },
                };
                let f = self.from_state;
                for c in self.output.clone() {
                    _ = c.send(f(i.clone()) + v.clone())
                }
                None
            }
        };
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
