use std::{any::Any, fmt::Debug, rc::Rc};

use crate::{
    connection::{Output, RuntimeConnectable},
    node::{State, UpdateError},
    nodes::node::Node,
};

use super::node::Context;

pub struct BasicNode<I>
where
    I: Clone,
{
    name: String,
    _state: State<Option<I>>,
    props: I,
    _context: State<Context>,

    pub output: Output<I>,
}

impl<I> BasicNode<I>
where
    I: Clone,
{
    pub fn new(name: &str, context: State<Context>, props: I) -> Self {
        Self {
            name: name.into(),
            _state: State::new(None),
            props,
            _context: context,
            output: Output::new(),
        }
    }
}

impl<I> Node for BasicNode<I>
where
    I: Clone + Debug,
{
    fn on_init(&self) {
        ()
    }

    fn on_ready(&self) {
        let elem = &self.props;
        self.output.clone().send(elem.clone()).unwrap();
    }

    fn on_shutdown(&self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&self) -> Result<(), UpdateError> {
        Ok(())
    }
}

impl<I: Clone + 'static> RuntimeConnectable for BasicNode<I> {
    fn input_at(&self, _: usize) -> Rc<dyn Any> {
        panic!("Index out of bounds for BasicNode")
    }

    fn output_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => {
                let re = self.output.clone();
                Rc::new(re)
            }
            _ => panic!("Intex out of bounds for BasicNode"),
        }
    }
}
