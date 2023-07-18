use std::fmt::Debug;
use std::sync::Arc;

use flow_derive::build_job;
use serde::Deserialize;

use crate::{
    job::{Context, Job},
    log,
};
use crate::{Connectable, Node};

#[derive(Connectable, Deserialize)]
pub struct DebugNode<I, O>
where
    I: Sized + Clone,
    O: Sized + Clone,
{
    conn: Connection<I, O>,
    _context: Arc<Context>,
    name: String,
}

impl<I, O> DebugNode<I, O>
where
    I: Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>) -> Self {
        let conn = Connection::new(1);
        Self {
            conn,
            name: name.into(),
            _context: context,
        }
    }
}

#[build_job]
impl<I, O> Job for DebugNode<I, O>
where
    I: Debug + Clone,
    O: Clone,
{
    fn handle(next_elem: I) {
        let msg = format!("{:?}", next_elem.clone());
        println!("{}", msg);
        log(msg.as_str());
    }
}

impl<I, O> Node<I, O> for DebugNode<I, O>
where
    I: Clone + Debug,
    O: Clone,
{
    fn on_init(&mut self) {
        ()
    }

    fn on_ready(&mut self) {
        ()
    }

    fn on_shutdown(&mut self) {
        ()
    }
}
