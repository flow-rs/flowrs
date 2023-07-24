use std::sync::Arc;

use serde::Deserialize;

use crate::job::{Context, Job};

#[derive(Deserialize)]
pub struct RepeatNode<I>
where
    I: Iterator + Clone,
{
    _context: Arc<Context>,
    name: String,
    pub state: Option<I>,
}

impl<I, O> RepeatNode<I>
where
    I: Iterator<Item = O> + Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>) -> Self {
        Self {
            state: None,
            name: name.into(),
            _context: context,
        }
    }
}

