#[cfg(test)]
mod nodes {
    use flow::connection::{ConnectError, Edge};
    use flow::job::Context;
    use flow::Node;
    use serde_json::Value;
    use std::sync::Arc;

    use flow::add::AddNode;

    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = Arc::new(Context {});
        let mut add = AddNode::new("AddNodeI32", context, Value::Null);
        let mock_output = Edge::new();
        add.connect(mock_output.clone());
        let _ = add.input_1.send(1);
        let _ = add.input_2.send(2);
        add.update();
        add.update();

        let expected = 3;
        let actual = mock_output.next_elem()?;
        Ok(assert!(expected == actual))
    }
}
