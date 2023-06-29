use std::{
    fmt::Display,
    sync::{mpsc::Sender, Arc},
};

use crate::Connectable;

use crate::{job::Job, log};

use super::connection::Connection;
use super::job::Context;

#[derive(Connectable)]
pub struct DebugNode<I, O>
where
    I: Sized,
    O: Sized,
{
    pub conn: Connection<I, O>,
    context: Arc<Context>,
    name: String,
}

impl<I, O> DebugNode<I, O> {
    pub fn new(name: &str, context: Arc<Context>) -> Self {
        let conn = Connection::new(1);
        Self {
            conn,
            name: name.into(),
            context,
        }
    }
}

impl<I, O> Job for DebugNode<I, O>
where
    I: Display + Clone,
    O: Clone,
{
    fn handle(&mut self) {
        print!("{} node prints: ", self.name);
        self.conn.input.iter().for_each(|c| {
            // Avoiding recv_timout since wasm can't access system time without JS bindings
            let msg = match c.try_recv() {
                Err(_) => format!("Nothing"),
                Ok(x) => format!("{}", x),
            };
            // Log to Rust and Browser console
            println!("{}", msg);
            log(msg.as_str());
        });
    }

    fn name(&self) -> &String {
        &self.name
    }
}
