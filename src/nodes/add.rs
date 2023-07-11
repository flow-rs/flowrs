use std::ops::Add;
use std::sync::Arc;

use flow_derive::build_job;
use serde::Deserialize;

use crate::job::{Context, Job};
use crate::{Connectable, Node};

#[derive(Connectable, Deserialize)]
pub struct AddNode<I, O>
where
    I: Sized + Clone,
    O: Clone
{
    conn: Connection<I, O>,
    _context: Arc<Context>,
    name: String,
    pub state: Option<I>,
}

impl<I, O> AddNode<I, O>
where
    I: Clone,
    O: Clone
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

#[build_job]
impl<I, O> Job for AddNode<I, O>
where
    I: Add<Output = O> + Clone,
    O: Clone
{
    fn handle_lhs(next_elem: I) {
        self.state = Some(next_elem);
    }

    fn handle_rhs(next_elem: I) {
        if let Some(input) = &self.state {
            self.send_out(input.clone() + next_elem.clone());
            self.state = None;
        }
    }
}

impl<I, O> Node<I, O> for AddNode<I, O>
where
    I: Add<Output = O> + Clone,
    O: Clone,
{
    fn init(&mut self) {
        ()
    }

    fn shutdown(&mut self) {
        ()
    }
}
