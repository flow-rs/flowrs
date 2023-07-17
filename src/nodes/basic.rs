use std::sync::Arc;

use flow_derive::build_job;
use serde::Deserialize;

use crate::job::{Context, Job};
use crate::{Connectable, Node};

#[derive(Connectable, Deserialize)]
pub struct BasicNode<I, O>
where
    I: Sized + Clone,
    O: Sized + Clone,
{
    conn: Connection<I, O>,
    _context: Arc<Context>,
    props: O,
    name: String,
}

impl<I, O> BasicNode<I, O>
where
    I: Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: O) -> Self {
        let conn = Connection::new(1);
        Self {
            conn,
            name: name.into(),
            _context: context,
            props
        }
    }
}

#[build_job]
impl<I, O> Job for BasicNode<I, O>
where
    I: Clone,
    O: Clone,
{
    fn handle(_next_elem: I) {
        ()
    }
}

impl<I, O> Node<I, O> for BasicNode<I, O>
where
    I: Clone,
    O: Clone,
{
    fn on_init(&mut self) {
        ()
    }

    fn on_ready(&mut self) {
        self.send_out(self.props.clone())
    }

    fn on_shutdown(&mut self) {
        ()
    }
}
