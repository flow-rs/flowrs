use std::{any::Any, fmt::Debug, rc::Rc, sync::Arc};

use crate::{
    connection::{Output, RuntimeConnectable},
    nodes::node::Node,
};

use super::node::Context;

pub struct BasicNode<I>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: I,
    context: Arc<Context>,

    pub output: Output<I>,
}

impl<I> BasicNode<I>
where
    I: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: I) -> Self {
        Self {
            name: name.into(),
            state: None,
            props,
            context,
            output: Output::new(),
        }
    }
}

impl<I> Node for BasicNode<I>
where
    I: Clone + Debug,
{
    fn on_init(&mut self) {
        ()
    }

    fn on_ready(&mut self) {
        let elem = &self.props;
        self.output.send(elem.clone()).unwrap();
    }

    fn on_shutdown(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self) {}
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
