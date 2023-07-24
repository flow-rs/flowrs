use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, Mutex}, rc::Rc,
};

use crate::{job::RuntimeConnectable, nodes::job::Node};

use super::{connection::Edge, job::Context};

pub struct BasicNode<I>
where
    I: Clone,
{
    name: String,
    state: Option<I>,
    props: I,
    context: Arc<Context>,

    pub output: Arc<Mutex<Option<Edge<I>>>>,
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
            output: Arc::new(Mutex::new(None)),
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
        self.output
            .lock()
            .unwrap()
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
}

impl<I: Clone + 'static> RuntimeConnectable for BasicNode<I> {
    fn input_at(&self, _: usize) -> Rc<dyn Any> {
        panic!("Index out of bounds for BasicNode")
    }

    fn output_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => {let re = self.output.clone(); Rc::new(re)},
            _ => panic!("Intex out of bounds for BasicNode"),
        }
    }
}
