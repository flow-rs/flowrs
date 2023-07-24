use std::any::Any;
use std::ops::Add;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::job::RuntimeConnectable;

use super::{
    connection::Edge,
    job::{Context, Node},
};

enum AddNodeState<I> {
    I1(I),
    I2(I),
    None,
}

pub struct AddNode<I, O> {
    name: String,
    state: AddNodeState<I>,
    props: Value,
    context: Arc<Context>,

    pub input_1: Edge<I>,
    pub input_2: Edge<I>,
    pub output_1: Arc<Mutex<Option<Edge<O>>>>,
}

impl<I, O> AddNode<I, O>
where
    I: Clone + Add<Output = O>,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            state: AddNodeState::None,
            props,
            context,

            output_1: Arc::new(Mutex::new(None)),
            input_1: Edge::new(),
            input_2: Edge::new(),
        }
    }

    fn handle_1(&mut self, v: I) {
        match &self.state {
            AddNodeState::I1(_) => panic!("Error, same input queue (1) was scheduled twice."),
            AddNodeState::I2(i) => {
                let out = i.clone() + v;
                self.state = AddNodeState::None;
                let _ = self.output_1.lock().unwrap().clone().expect("This node has no Output.").send(out);
            }
            AddNodeState::None => self.state = AddNodeState::I1(v),
        }
    }

    fn handle_2(&mut self, v: I) {
        match &self.state {
            AddNodeState::I2(_) => panic!("Error, same input queue (2) was scheduled twice."),
            AddNodeState::I1(i) => {
                let out = i.clone() + v;
                self.state = AddNodeState::None;
                let _ = self.output_1.lock().unwrap().clone().expect("This node has no Output.").send(out);
            }
            AddNodeState::None => self.state = AddNodeState::I2(v),
        }
    }
}

impl<I, O> Node for AddNode<I, O>
where
    I: Add<Output = O> + Clone,
    O: Clone,
{
    fn on_init(&mut self) {}

    fn on_ready(&mut self) {}

    fn on_shutdown(&mut self) {}

    fn name(&self) -> &str {
        &self.name
    }

    // To be replaced by macro
    fn update(&mut self) {
        if let Ok(i1) = self.input_1.next_elem() {
            println!("TEST1");
            self.handle_1(i1);
        }

        if let Ok(i2) = self.input_2.next_elem() {
            println!("TEST2");
            self.handle_2(i2);
        }
    }
}

// To be replaced by macro
impl<I: Clone + 'static, O: Clone + 'static> RuntimeConnectable for AddNode<I, O> {
    fn input_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.input_1.clone()),
            1 => Rc::new(self.input_2.clone()),
            _ => panic!("Intex out of bounds for AddNode")
        }
    }

    fn output_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.output_1.clone()),
            _ => panic!("Intex out of bounds for AddNode")
        }
    }
}
