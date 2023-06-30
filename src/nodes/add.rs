use std::ops::Add;
use std::sync::Arc;

use crate::job::{Context, Job};
use crate::Connectable;

#[derive(Connectable)]
pub struct AddNode<I, O>
where
    I: Sized,
{
    conn: Connection<I, O>,
    name: String,
    context: Arc<Context>,
    pub state: Option<I>,
    neutral_ele: I,
}

impl<I, O> AddNode<I, O> {
    pub fn new(name: &str, context: Arc<Context>, neutral_ele: I) -> Self {
        let conn = Connection::new(2);
        Self {
            neutral_ele,
            conn,
            state: None,
            name: name.into(),
            context,
        }
    }
}

impl<I, O> Job for AddNode<I, O>
where
    I: Add<Output = O> + Clone,
    O: Clone,
{
    fn handle(&mut self) {
        self.state = match &self.state {
            None => match self.conn.input.get(0) {
                None => None,
                Some(i) => {
                    // Avoiding recv_timout since wasm can't access system time without JS bindings
                    match i.try_recv() {
                        Err(_) => return,
                        Ok(value) => Some(value),
                    }
                }
            },
            Some(i) => {
                // TODO: don't use conn at all
                let recv = self.conn.input.get(1);
                let value = match recv {
                    None => self.neutral_ele.clone(),
                    // Avoiding recv_timout since wasm can't access system time without JS bindings
                    Some(chan) => match chan.try_recv() {
                        Ok(value) => value,
                        // Nothing to do, skipping cycle
                        Err(_) => return,
                    },
                };
                for chan in self.output() {
                    _ = chan.send(i.clone() + value.clone())
                }
                None
            }
        };
    }

    fn name(&self) -> &String {
        &self.name
    }
}
