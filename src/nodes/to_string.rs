use std::{fmt::Debug, rc::Rc, sync::Arc};

use crate::{flow::app_state::FlowType, job::RuntimeConnectable, nodes::job::Node};

use super::{connection::Edge, job::Context};

pub struct ToStringNode<I, O1, O2>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: I,
    context: Arc<Context>,

    pub input: Edge<I>,
    pub output_1: Option<Edge<O1>>,
    pub output_2: Option<Edge<O2>>,
}

impl<I, O1, O2> ToStringNode<I, O1, O2>
where
    I: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: I) -> Self {
        Self {
            name: name.into(),
            state: None,
            props,
            context,
            input: Edge::new(),
            output_1: None,
            output_2: None,
        }
    }
}

enum Out<O1, O2> {
    Out1(O1),
    Out2(O2)
}

impl<I, O1, O2> Node for ToStringNode<I, O1, O2>
where
    I: Clone + Debug,
    O1: ToString,
    O2: ToString,
{
    type Output = Out<O1, O2>;
    fn on_init(&mut self) {}

    fn on_ready(&mut self) {}

    fn on_shutdown(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }

    fn update(&mut self) {
        if let Ok(input) = self.input.next_elem() {
            println!("{:?}", input);
        }
    }

    fn connect<T>(&mut self, edge_in: Edge<T>, edge_out: Edge<T>) {
        match T {
            
        };
    }
}

impl<I: Clone + 'static, O1: Clone + 'static, O2: Clone + 'static> RuntimeConnectable for ToStringNode<I, O1, O2> {
    fn input_at(&self, index: usize) -> FlowType {
        match index {
            0 => FlowType(Rc::new(self.input.clone())),
            _ => panic!("Intex out of bounds for DebugNode"),
        }
    }

    fn output_at(&self, index: usize) -> FlowType {
        match index {
            0 => FlowType(Rc::new(self.output_1.clone())),
            1 => FlowType(Rc::new(self.output_2.clone())),
            _ => panic!("Intex out of bounds for DebugNode"),
        }
    }
}
