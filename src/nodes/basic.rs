use std::{fmt::Debug, rc::Rc, sync::Arc};

use crate::{flow::app_state::FlowType, job::RuntimeConnectable, nodes::job::Node};

use super::{connection::Edge, job::Context};

pub struct BasicNode<I>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: I,
    context: Arc<Context>,

    pub output: Option<Edge<I>>,
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
            output: None,
        }
    }
}

impl<I> Node for BasicNode<I>
where
    I: Clone + Debug,
{
    type Output = I;
    fn on_init(&mut self) {
        ()
    }

    fn on_ready(&mut self) {
        let elem = &self.props;
        self.output
            .clone()
            .expect("This Basic Node has no successor.")
            .send(elem.clone())
            .unwrap();
    }

    fn on_shutdown(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self) {}

    fn connect(&mut self, edge: Edge<I>) {
        self.output = Some(edge)
    }
}

impl<I: Clone + 'static> RuntimeConnectable for BasicNode<I> {
    fn input_at(&self, index: usize) -> FlowType {
        panic!("Index out of bounds for BasicNode")
    }

    fn output_at(&self, index: usize) -> FlowType {
        match index {
            0 => FlowType(Rc::new(self.output.clone().unwrap())),
            _ => panic!("Intex out of bounds for BasicNode"),
        }
    }
}
