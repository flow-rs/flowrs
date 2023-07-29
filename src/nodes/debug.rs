use std::{any::Any, fmt::Debug, rc::Rc};

use serde_json::Value;

use crate::{
    connection::{Input, Output, RuntimeConnectable},
    node::{State, UpdateError},
    nodes::node::Node,
};

use super::node::Context;

pub struct DebugNode<I>
where
    I: Clone,
{
    name: String,
    _state: State<Option<I>>,
    _props: Value,
    _context: State<Context>,

    pub input: Input<I>,
    pub output: Output<I>,
}

impl<I> DebugNode<I>
where
    I: Clone,
{
    pub fn new(name: &str, context: State<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            _state: State::new(None),
            _props: props,
            _context: context,
            input: Input::new(),
            output: Output::new(),
        }
    }
}

impl<I> Node for DebugNode<I>
where
    I: Clone + Debug,
{
    fn on_init(&self) {}

    fn on_ready(&self) {}

    fn on_shutdown(&self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&self) -> Result<(), UpdateError> {
        if let Ok(input) = self.input.next_elem() {
            println!("{:?}", input);
            self.output.clone().send(input).unwrap();
        }
        Ok(())
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
