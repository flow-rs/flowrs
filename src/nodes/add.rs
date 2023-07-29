use std::any::Any;
use std::ops::Add;
use std::rc::Rc;

use flow_derive::Connectable;
use serde_json::Value;

use super::node::{Context, Node};
use crate::connection::{Input, Output, RuntimeConnectable};
use crate::node::{SequenceError, State, UpdateError};

#[derive(Clone)]
enum AddNodeState<I1, I2> {
    I1(I1),
    I2(I2),
    None,
}

#[derive(Connectable)]
pub struct AddNode<I1, I2, O>
where
    I1: Clone,
    I2: Clone,
{
    name: String,
    state: State<AddNodeState<I1, I2>>,
    _props: Value,
    _context: State<Context>,

    #[input]
    pub input_1: Input<I1>,
    #[input]
    pub input_2: Input<I2>,
    #[output]
    pub output_1: Output<O>,
}

impl<I1, I2, O> AddNode<I1, I2, O>
where
    I1: Clone + Add<I2, Output = O>,
    I2: Clone,
    O: Clone,
{
    pub fn new(name: &str, context: State<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            state: State::new(AddNodeState::None),
            _props: props,
            _context: context,

            input_1: Input::new(),
            input_2: Input::new(),
            output_1: Output::new(),
        }
    }

    fn handle_1(&self, v: I1) -> Result<(), UpdateError> {
        let mut state = self.state.0.lock().unwrap();
        match state.clone() {
            AddNodeState::I1(_) => {
                return Err(UpdateError::SequenceError(SequenceError {
                    node: self.name().into(),
                    message: "Addition should happen pairwise.".into(),
                }))
            }
            AddNodeState::I2(i) => {
                let out = v + i.clone();
                *state = AddNodeState::None;
                let _ = self.output_1.clone().send(out);
            }
            AddNodeState::None => *state = AddNodeState::I1(v),
        }
        Ok(())
    }

    fn handle_2(&self, v: I2) -> Result<(), UpdateError> {
        let mut state = self.state.0.lock().unwrap();
        match state.clone() {
            AddNodeState::I2(_) => {
                return Err(UpdateError::SequenceError(SequenceError {
                    node: self.name().into(),
                    message: "Addition should happen pairwise.".into(),
                }))
            }
            AddNodeState::I1(i) => {
                let out = i.clone() + v;
                *state = AddNodeState::None;
                let _ = self.output_1.clone().send(out);
            }
            AddNodeState::None => *state = AddNodeState::I2(v),
        }
        Ok(())
    }
}

impl<I1, I2, O> Node for AddNode<I1, I2, O>
where
    I1: Add<I2, Output = O> + Clone,
    I2: Clone,
    O: Clone,
{
    fn on_init(&self) {}

    fn on_ready(&self) {}

    fn on_shutdown(&self) {}

    fn name(&self) -> &str {
        &self.name
    }

    // To be replaced by macro
    fn update(&self) -> Result<(), UpdateError> {
        if let Ok(i1) = self.input_1.next_elem() {
            self.handle_1(i1)?;
        }

        if let Ok(i2) = self.input_2.next_elem() {
            self.handle_2(i2)?;
        }
        Ok(())
    }
}
