use std::ops::Add;
use std::sync::{mpsc::Sender, Arc};

use crate::Connectable;

use crate::job::Job;

use super::connection::Connection;
use super::job::Context;

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
                        Ok(v) => Some(v),
                    }
                }
            },
            Some(i) => {
                let v1 = self.conn.input.get(1);
                let v = match v1 {
                    None => self.neutral_ele.clone(),
                    // Avoiding recv_timout since wasm can't access system time without JS bindings
                    Some(c) => match c.try_recv() {
                        Ok(w) => w,
                        // Nothing to do, skipping cycle
                        Err(_) => return,
                    },
                };
                for c in self.conn.output() {
                    _ = c.send(i.clone() + v.clone())
                }
                None
            }
        };
    }

    fn name(&self) -> &String {
        &self.name
    }
}
