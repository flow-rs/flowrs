use std::any::Any;
use std::ops::Add;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::Value;

use crate::connection::RuntimeConnectable;
use crate::connection::{Input, Output};

use super::node::{Context, Node};

enum AddNodeState<I1, I2> {
    I1(I1),
    I2(I2),
    None,
}

pub struct AddNode<I1, I2, O> {
    name: String,
    state: AddNodeState<I1, I2>,
    props: Value,
    context: Arc<Context>,

    pub input_1: Input<I1>,
    pub input_2: Input<I2>,
    pub output_1: Output<O>,
}

impl<I1, I2, O> AddNode<I1, I2, O>
where
    I1: Clone + Add<I2, Output = O>,
    I2: Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            state: AddNodeState::None,
            props,
            context,

            input_1: Input::new(),
            input_2: Input::new(),
            output_1: Output::new(),
        }
    }

    fn handle_1(&mut self, v: I1) {
        match &self.state {
            AddNodeState::I1(_) => panic!("Error, same input queue (1) was scheduled twice."),
            AddNodeState::I2(i) => {
                let out = v + i.clone();
                self.state = AddNodeState::None;
                let _ = self.output_1.send(out);
            }
            AddNodeState::None => self.state = AddNodeState::I1(v),
        }
    }

    fn handle_2(&mut self, v: I2) {
        match &self.state {
            AddNodeState::I2(_) => panic!("Error, same input queue (2) was scheduled twice."),
            AddNodeState::I1(i) => {
                let out = i.clone() + v;
                self.state = AddNodeState::None;
                let _ = self.output_1.send(out);
            }
            AddNodeState::None => self.state = AddNodeState::I2(v),
        }
    }
}

impl<I1, I2, O> Node for AddNode<I1, I2, O>
where
    I1: Add<I2, Output = O> + Clone,
    I2: Clone,
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
impl<I1, I2, O> RuntimeConnectable for AddNode<I1, I2, O>
where
    I1: Clone + 'static,
    I2: Clone + 'static,
    O: Clone + 'static,
{
    fn input_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.input_1.clone()),
            1 => Rc::new(self.input_2.clone()),
            _ => panic!("Intex out of bounds for AddNode"),
        }
    }

    fn output_at(&self, index: usize) -> Rc<dyn Any> {
        match index {
            0 => Rc::new(self.output_1.clone()),
            _ => panic!("Intex out of bounds for AddNode"),
        }
    }
}
