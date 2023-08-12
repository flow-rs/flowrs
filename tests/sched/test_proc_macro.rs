use std::fmt::Display;

use flowrs::{
    connection::{Input, Output},
    node::{ChangeObserver, Node},
};
use flowrs_derive::RuntimeConnectable;

#[derive(RuntimeConnectable)]
pub struct PassthroughNode {
    #[input]
    input: Input<i32>,
    #[output]
    output: Output<i32>,
}
impl PassthroughNode {
    fn new(change_observer: &ChangeObserver) -> Self {
        Self {
            input: Input::new(),
            output: Output::new(Some(change_observer)),
        }
    }
}
impl Node for PassthroughNode {}

#[derive(RuntimeConnectable)]
pub struct MultiNode<T> {
    #[input]
    input: Input<i32>,
    #[output]
    output_str: Output<String>,
    #[input]
    input2: Input<Vec<u128>>,
    #[output]
    output: Output<T>,
    #[output]
    a: Output<Box<String>>
}
impl<T> MultiNode<T>
where
    T: Clone,
{
    fn new(change_observer: &ChangeObserver) -> Self {
        Self {
            input: Input::new(),
            output: Output::new(Some(change_observer)),
            output_str: Output::new(Some(change_observer)),
            input2: Input::new(),
            a: Output::new(Some(change_observer)),
        }
    }
}
impl<T> Node for MultiNode<T> where T: Clone + Send {}

#[cfg(test)]
mod test_execution {
    use std::{any::Any, collections::HashMap, rc::Rc};

    use flowrs::{
        connection::{Input, Output, RuntimeConnectable},
        node::{ChangeObserver, Node},
    };
    use flowrs_derive::RuntimeConnectable;

    use crate::sched::test_proc_macro::{MultiNode, PassthroughNode};

    #[test]
    #[should_panic(expected = "Index 0 out of bounds for EmptyNode with input len 0.")]
    fn should_fail_on_index_zero() {
        #[derive(RuntimeConnectable)]
        pub struct EmptyNode {}
        impl EmptyNode {
            fn new() -> Self {
                Self {}
            }
        }
        impl Node for EmptyNode {}
        let empty_node = EmptyNode::new();
        empty_node.input_at(0);
    }

    #[test]
    #[should_panic(expected = "Index 0 out of bounds for EmptyInputNode with input len 0.")]
    fn should_ignore_unannotated_input() {
        #[derive(RuntimeConnectable)]
        pub struct EmptyInputNode {
            _input: Input<String>,
        }
        impl EmptyInputNode {
            fn new() -> Self {
                Self {
                    _input: Input::new(),
                }
            }
        }
        impl Node for EmptyInputNode {}
        let empty_input_node = EmptyInputNode::new();
        empty_input_node.input_at(0);
    }

    #[test]
    #[should_panic(expected = "Index 0 out of bounds for InvalidInputNode with input len 0.")]
    fn should_ignore_invalid_input() {
        #[derive(RuntimeConnectable)]
        pub struct InvalidInputNode {
            _input: String,
        }
        impl InvalidInputNode {
            fn new() -> Self {
                Self {
                    _input: "".to_string(),
                }
            }
        }
        impl Node for InvalidInputNode {}
        let invalid_input_node = InvalidInputNode::new();
        invalid_input_node.input_at(0);
    }

    #[test]
    fn should_return_input_output_at_runtime() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: PassthroughNode = PassthroughNode::new(&change_observer);
        let input1: Result<_, Rc<dyn Any>> = passthrough_node.input_at(0).downcast::<Input<i32>>();
        let output1: Result<_, Rc<dyn Any>> =
            passthrough_node.output_at(0).downcast::<Output<i32>>();
        assert!(input1.is_ok());
        assert!(output1.is_ok())
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for PassthroughNode with input len 1.")]
    fn should_fail_on_input_out_of_bounds() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: PassthroughNode = PassthroughNode::new(&change_observer);
        passthrough_node.input_at(1);
    }

    #[test]
    #[should_panic(expected = "Index 1 out of bounds for PassthroughNode with output len 1.")]
    fn should_fail_on_output_out_of_bounds() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: PassthroughNode = PassthroughNode::new(&change_observer);
        passthrough_node.output_at(1);
    }

    #[test]
    fn should_return_generic_input_output_at_runtime() {
        #[derive(RuntimeConnectable)]
        pub struct GenericNode<T> {
            #[input]
            input: Input<T>,
            #[output]
            output: Output<T>,
        }
        impl<T> GenericNode<T> {
            fn new(change_observer: &ChangeObserver) -> Self {
                Self {
                    input: Input::new(),
                    output: Output::new(Some(change_observer)),
                }
            }
        }
        impl<T: Send> Node for GenericNode<T> {}

        let change_observer: ChangeObserver = ChangeObserver::new();

        let generic_node: GenericNode<u8> = GenericNode::new(&change_observer);
        let input1: Result<_, Rc<dyn Any>> = generic_node.input_at(0).downcast::<Input<u8>>();
        let output1: Result<_, Rc<dyn Any>> = generic_node.output_at(0).downcast::<Output<u8>>();
        assert!(input1.is_ok());
        assert!(output1.is_ok())
    }

    #[test]
    fn should_return_nested_generic_input_output_at_runtime() {
        #[derive(RuntimeConnectable)]
        pub struct GenericNestedNode<T, S> {
            #[input]
            input: Input<Vec<T>>,
            #[output]
            output: Output<HashMap<u8, S>>,
        }
        impl<T, S> GenericNestedNode<T, S> {
            fn new(change_observer: &ChangeObserver) -> Self {
                Self {
                    input: Input::new(),
                    output: Output::new(Some(change_observer)),
                }
            }
        }
        impl<T: Send, S: Send> Node for GenericNestedNode<T, S> {}
        let change_observer: ChangeObserver = ChangeObserver::new();

        let nested_generic_node: GenericNestedNode<u8, Vec<String>> =
            GenericNestedNode::new(&change_observer);
        let input1: Result<_, Rc<dyn Any>> =
            nested_generic_node.input_at(0).downcast::<Input<Vec<u8>>>();
        let output1: Result<_, Rc<dyn Any>> = nested_generic_node
            .output_at(0)
            .downcast::<Output<HashMap<u8, Vec<String>>>>();
        assert!(input1.is_ok());
        assert!(output1.is_ok())
    }

    #[test]
    fn should_return_multiple_input_output_at_runtime() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let nested_generic_node: MultiNode<[u8; 20]> = MultiNode::new(&change_observer);
        let input1: Result<_, Rc<dyn Any>> =
            nested_generic_node.input_at(0).downcast::<Input<i32>>();
        let input2: Result<_, Rc<dyn Any>> = nested_generic_node
            .input_at(1)
            .downcast::<Input<Vec<u128>>>();
        let output1: Result<_, Rc<dyn Any>> = nested_generic_node
            .output_at(0)
            .downcast::<Output<String>>();
        let output2: Result<_, Rc<dyn Any>> = nested_generic_node
            .output_at(1)
            .downcast::<Output<[u8; 20]>>();
        assert!(input1.is_ok());
        assert!(output1.is_ok());
        assert!(input2.is_ok());
        assert!(output2.is_ok())
    }

    #[test]
    #[should_panic(expected = "Index 2 out of bounds for MultiNode with input len 2.")]
    fn should_fail_on_mult_input_out_of_bounds() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: MultiNode<u8> = MultiNode::new(&change_observer);
        passthrough_node.input_at(2);
    }

    #[test]
    #[should_panic(expected = "Index 4 out of bounds for MultiNode with output len 3.")]
    fn should_fail_on_mult_output_out_of_bounds() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: MultiNode<u8> = MultiNode::new(&change_observer);
        passthrough_node.output_at(4);
    }

    #[test]
    #[should_panic(expected = "Index 3 out of bounds for MultiNode with output len 3.")]
    fn should_fail_on_mult_output_out_of_bounds_at_three() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let passthrough_node: MultiNode<u8> = MultiNode::new(&change_observer);
        passthrough_node.output_at(3);
    }
}
