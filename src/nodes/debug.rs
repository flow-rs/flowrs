use std::{fmt::Display, sync::Arc};

use flow_derive::build_job;
use serde::Deserialize;

use crate::Connectable;
use crate::{
    job::{Context, Job},
    log,
};

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
    I: Display + Clone,
    O: Clone,
{
    fn handle(next_elem: I) {
        let msg = format!("{}", next_elem.clone());
        println!("{}", msg);
        log(msg.as_str());
    }
}
