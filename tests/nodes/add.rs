#[cfg(test)]
mod nodes {
    use flow::connection::{connect, ConnectError, Edge, Input, Output, RuntimeConnectable};
    use flow::node::{Context, Node, State};
    use serde_json::Value;
    use std::any::Any;
    use std::rc::Rc;
    use std::{thread, vec};

    use flow::add::AddNode;

    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = State::new(Context {});
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
        let context = State::new(Context {});
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
        let context = State::new(Context {});
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
        let context = State::new(Context {});
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.input_at(0);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_rhs_at_runtime() {
        let context = State::new(Context {});
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.input_at(1);
        let input1_downcasted = input1.downcast::<Input<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    fn should_return_output_at_runtime() {
        let context = State::new(Context {});
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        let input1: Rc<dyn Any> = add.output_at(0);
        let input1_downcasted = input1.downcast::<Output<i32>>();
        assert!(input1_downcasted.is_ok())
    }

    #[test]
    #[should_panic(expected = "Index 2 out of bounds for AddNode with input len 2.")]
    fn should_fail_on_index_out_of_bounds() {
        let context = State::new(Context {});
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        add.input_at(2);
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for AddNode with output len 1.")]
    fn should_fail_on_output_out_of_bounds() {
        let context = State::new(Context {});
        let add: AddNode<i32, i32, i32> = AddNode::new("AddNodeI32", context.clone(), Value::Null);
        add.output_at(1);
    }
}
