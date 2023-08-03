use std::any::Any;
use std::ops::Add;
use std::rc::Rc;

use serde_json::Value;

use flowrs::{
    connection::{Input, Output, RuntimeConnectable},
    node::{Context, Node, State, UpdateError, InitError, ShutdownError, ReadyError},
};
use flowrs_derive::Connectable;

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
    I1: Clone + Add<I2, Output = O> + Send + 'static,
    I2: Clone + Send + 'static,
    O: Clone + Send + 'static,
{
    pub fn new(name: &str, context: State<Context>, props: Value) -> Self {
        Self {
            name: name.into(),
            state: State::new(AddNodeState::None),
            _props: props,
            _context: context.clone(),

            input_1: Input::new(),
            input_2: Input::new(),
            output_1: Output::new(context.clone()),
        }
    }

    fn handle_1(&self, v: I1) -> Result<(), UpdateError> {
        let mut state = self.state.0.lock().unwrap();
        match state.clone() {
            AddNodeState::I1(_) => {
                return Err(UpdateError::SequenceError {
                    node: self.name().into(),
                    message: "Addition should happen pairwise.".into(),
                })
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
                return Err(UpdateError::SequenceError {
                    node: self.name().into(),
                    message: "Addition should happen pairwise.".into(),
                })
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
    I1: Add<I2, Output = O> + Clone + Send + 'static,
    I2: Clone + Send + 'static,
    O: Clone + Send + 'static,
{
    fn on_init(&self) -> Result<(), InitError>{ 
        Ok(())
    }

    fn on_ready(&self)   -> Result<(), ReadyError>{
        Ok(())
    }

    fn on_shutdown(&self)  -> Result<(), ShutdownError> {
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    // To be replaced by macro
    fn update(&self) -> Result<(), UpdateError> {
        if let Ok(i1) = self.input_1.next_elem() {
            println!("UPDATE1");
            self.handle_1(i1)?;
        }

        if let Ok(i2) = self.input_2.next_elem() {
            println!("UPDATE2");
            self.handle_2(i2)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod nodes {
    use std::{thread, rc::Rc, any::Any};

    use flowrs::{connection::{ConnectError, Edge, connect, Input, RuntimeConnectable, Output}, node::{Context, State, Node}};
    use serde_json::Value;

    use super::AddNode;



    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = State::new(Context::new());
        let add = AddNode::new("AddNodeI32", context, Value::Null);
        let mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        let _ = add.input_1.send(1);
        let _ = add.input_2.send(2);
        let _ = add.update();
        let _ = add.update();

        let expected = 3;
        let actual = mock_output.next_elem()?;
        Ok(assert!(expected == actual))
    }

    /// Scenario:
    ///
    /// [0, 1, ..., 100]
    ///         \
    ///          >-<Add>--[100, 100, ..., 100]
    ///         /
    /// [100, 99, ..., 0]
    #[test]
    fn should_add_multiple_132_sequentially() -> Result<(), ConnectError<i32>> {
        let context = State::new(Context::new());
        let add = AddNode::new("AddNodeI32", context, Value::Null);
        let mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        (0..100).for_each(|int| {
            let _ = add.input_1.send(int);
        });
        (0..101).rev().for_each(|int| {
            let _ = add.input_2.send(int);
        });
        (0..100).for_each(|_| {
            let _ = add.update();
        });
        let mut actual = vec![];
        for _ in 0..100 {
            let curr = mock_output.next_elem()?;
            actual.push(curr)
        }
        let exected = vec![100; 100];
        Ok(assert!(
            exected == actual,
            "expected was: {:?} while actual was {:?}",
            exected,
            actual
        ))
    }

    #[test]
    fn should_add_multiple_132_parallel() -> Result<(), ConnectError<i32>> {
        let context = State::new(Context::new());
        let add1 = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let add2 = AddNode::new("AddNodeI32", context, Value::Null);
        let mock_output = Edge::new();
        connect(add1.output_1.clone(), add2.input_1.clone());
        connect(add2.output_1.clone(), mock_output.clone());
        (0..100).for_each(|int| {
            let _ = add1.input_1.send(int);
        });
        (0..101).rev().for_each(|int| {
            let _ = add1.input_2.send(int);
        });
        (0..100).rev().for_each(|_| {
            let _ = add2.input_2.send(1);
        });

        let handle1 = thread::spawn(move || {
            (0..100).for_each(|_| {
                match add1.update() {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        });
        let handle2 = thread::spawn(move || {
            (0..100).for_each(|_| {
                match add2.update() {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        let mut actual = vec![];
        for _ in 0..100 {
            let curr = mock_output.next_elem();
            actual.push(curr)
        }
        Ok(assert!(!actual.is_empty()))
    }

    #[test]
    fn should_return_lhs_at_runtime() {
        let context = State::new(Context::new());
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.input_at(0);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_rhs_at_runtime() {
        let context = State::new(Context::new());
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.input_at(1);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_output_at_runtime() {
        let context = State::new(Context::new());
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.output_at(0);
        let input1_downcasted = input1.downcast::<Output<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    #[should_panic(expected = "Index 2 out of bounds for AddNode with input len 2.")]
    fn should_fail_on_index_out_of_bounds() {
        let context = State::new(Context::new());
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        add.input_at(2);
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for AddNode with output len 1.")]
    fn should_fail_on_output_out_of_bounds() {
        let context = State::new(Context::new());
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        add.output_at(1);
    }
}