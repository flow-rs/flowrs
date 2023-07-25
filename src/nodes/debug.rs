use std::{any::Any, fmt::Debug, rc::Rc, sync::Arc};

use serde_json::Value;

use crate::{
    connection::{Input, Output, RuntimeConnectable},
    nodes::node::Node,
};

use super::node::Context;

pub struct DebugNode<I>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: Value,
    context: Arc<Context>,

    pub input: Input<I>,
    pub output: Output<I>,
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
            input: Input::new(),
            output: Output::new(),
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
            self.output.send(input).unwrap();
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
