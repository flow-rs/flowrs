use std::{fmt::Debug, sync::{Arc, Mutex}, any::Any, rc::Rc};

use serde_json::Value;

use crate::{job::RuntimeConnectable, nodes::job::Node};

use super::{connection::Edge, job::Context};

pub struct DebugNode<I>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: Value,
    context: Arc<Context>,

    pub input: Edge<I>,
    pub output: Arc<Mutex<Option<Edge<I>>>>,
}

impl<I> DebugNode<I>
where
    I: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            state: None,
            props,
            context,
            input: Edge::new(),
            output: Arc::new(Mutex::new(None)),
        }
    }
}

impl<I> Node for DebugNode<I>
where
    I: Clone + Debug,
{
    fn on_init(&mut self) {}

    fn on_ready(&mut self) {}

    fn on_shutdown(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self) {
        if let Ok(input) = self.input.next_elem() {
            println!("{:?}", input);
            self.output.lock().unwrap().clone().unwrap().send(input).unwrap();
        }
    }
}

impl<I: Clone + 'static> RuntimeConnectable for DebugNode<I> {
    fn input_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.input.clone()),
            _ => panic!("Intex out of bounds for DebugNode"),
        }
    }

    fn output_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.output.clone()),
            _ => panic!("Intex out of bounds for DebugNode"),
        }
    }
}
