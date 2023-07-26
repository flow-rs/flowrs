#[cfg(test)]
mod nodes {
    use flow::connection::{connect, ConnectError, Edge};
    use flow::node::{Context, Node};
    use serde_json::Value;
    use std::sync::Arc;
    use std::vec;

    use flow::add::AddNode;

    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = Arc::new(Context {});
        let mut add = AddNode::new("AddNodeI32", context, Value::Null);
        let mut mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        let _ = add.input_1.send(1);
        let _ = add.input_2.send(2);
        add.update();
        add.update();

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
        let context = Arc::new(Context {});
        let mut add = AddNode::new("AddNodeI32", context, Value::Null);
        let mut mock_output = Edge::new();
        connect(add.output_1.clone(), mock_output.clone());
        (0..100).for_each(|int| {
            let _ = add.input_1.send(int);
        });
        (0..101).rev().for_each(|int| {
            let _ = add.input_2.send(int);
        });
        (0..100).for_each(|_| add.update());
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
}
