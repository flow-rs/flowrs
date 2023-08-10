use std::ops::Add;

use flowrs::{
    connection::{Input, Output},
    node::{ChangeObserver, InitError, Node, ReadyError, ShutdownError, State, UpdateError},
};

use flowrs_derive::RuntimeConnectable;

#[derive(Clone)]
enum AddNodeState<I1, I2> {
    I1(I1),
    I2(I2),
    None,
}

#[derive(RuntimeConnectable)]
pub struct AddNode<I1, I2, O>
where
    I1: Clone,
    I2: Clone,
{
    name: String,
    state: State<AddNodeState<I1, I2>>,

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
    pub fn new(name: &str, change_observer: Option<&ChangeObserver>) -> Self {
        Self {
            name: name.into(),
            state: State::new(AddNodeState::None),

            input_1: Input::new(),
            input_2: Input::new(),
            output_1: Output::new(change_observer),
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
                self.output_1.clone().send(out);
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
                self.output_1.clone().send(out);
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
    

    fn name(&self) -> &str {
        &self.name
    }

    // To be replaced by macro
    fn on_update(&mut self) -> Result<(), UpdateError> {
        if let Ok(i1) = self.input_1.next() {
            println!("UPDATE1");
            self.handle_1(i1)?;
        }

        if let Ok(i2) = self.input_2.next() {
            println!("UPDATE2");
            self.handle_2(i2)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{any::Any, rc::Rc, thread};

    use anyhow::Error;
    use flowrs::{
        connection::{connect, Edge, Input, Output, RuntimeConnectable},
        node::{ChangeObserver, Node},
    };

    use super::AddNode;

    #[test]
    fn should_add_132() -> Result<(), Error> {
        let mut add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        let _ = add.input_1.send(1);
        let _ = add.input_2.send(2);
        let _ = add.on_update();
        let _ = add.on_update();

        let expected = 3;
        let actual = mock_output.next()?;
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
    fn should_add_multiple_132_sequentially() -> Result<(), Error> {
        let mut add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        (0..100).for_each(|int| {
            let _ = add.input_1.send(int);
        });
        (0..101).rev().for_each(|int| {
            let _ = add.input_2.send(int);
        });
        (0..100).for_each(|_| {
            let _ = add.on_update();
        });
        let mut actual = vec![];
        for _ in 0..100 {
            let curr = mock_output.next()?;
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
    fn should_add_multiple_132_parallel() -> Result<(), Error> {
        let mut add1 = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let mut add2 = AddNode::<i32, i32, i32>::new("AddNodeI32", None);

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
                match add1.on_update() {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        });
        let handle2 = thread::spawn(move || {
            (0..100).for_each(|_| {
                match add2.on_update() {
                    Ok(_) => (),
                    Err(e) => println!("{:?}", e),
                };
            });
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        let mut actual = vec![];
        for _ in 0..100 {
            let curr = mock_output.next();
            actual.push(curr)
        }
        Ok(assert!(!actual.is_empty()))
    }
    

    #[test]
    fn should_return_lhs_at_runtime() {
        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let input1: Rc<dyn Any> = add.input_at(0);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_rhs_at_runtime() {
        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let input1: Rc<dyn Any> = add.input_at(1);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_output_at_runtime() {
        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        let input1: Rc<dyn Any> = add.output_at(0);
        let input1_downcasted = input1.downcast::<Output<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    #[should_panic(expected = "Index 2 out of bounds for AddNode with input len 2.")]
    fn should_fail_on_index_out_of_bounds() {
        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        add.input_at(2);
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for AddNode with output len 1.")]
    fn should_fail_on_output_out_of_bounds() {
        let add = AddNode::<i32, i32, i32>::new("AddNodeI32", None);
        add.output_at(1);
    }
}

