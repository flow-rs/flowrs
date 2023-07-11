use std::sync::Arc;

use serde::Deserialize;

use crate::job::{Context, Job};
use crate::Connectable;

#[derive(Connectable, Deserialize)]
pub struct RepeatNode<I, O>
where
    I: Iterator + Clone,
    O: Clone
{
    conn: Connection<I, O>,
    _context: Arc<Context>,
    name: String,
    pub state: Option<I>,
}

impl<I, O> RepeatNode<I, O>
where
    I: Iterator<Item = O> + Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>) -> Self {
        let conn = Connection::new(2);
        Self {
            conn,
            state: None,
            name: name.into(),
            _context: context,
        }
    }
}

impl<I, O> Job for RepeatNode<I, O>
where
    I: Iterator<Item = O> + Clone,
    O: Clone,
{
    fn name(&self) -> &String {
        &self.name
    }

    fn handle(&mut self) {
        let new_state = match &self.state {
            // Sending each value of an input to each successor in a single handle to keep the pace.
            Some(iter) => {
                iter.clone().for_each(|entry| {
                    self.output()
                        .iter()
                        .for_each(|chan| chan.send(entry.clone()).unwrap())
                });
                None
            }
            None => match self.conn.input[0].try_recv(){
                Ok(input) => Some(input),
                Err(_) => None,
            },
        };
        self.state = new_state;
    }
}
