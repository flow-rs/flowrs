#[cfg(test)]
mod nodes {
    use std::sync::Arc;

    use flow::{basic::BasicNode, job::Context, connection::{Edge, ConnectError, connect}, Node};


    #[test]
    fn should_send_on_ready() -> Result<(), ConnectError<i32>> {
        let context = Arc::new(Context {});
        let mut node = BasicNode::new("My Node", context, 42);
        let mock_output = Edge::new();
        connect(node.output.clone(), mock_output.clone());
        node.on_ready();

        let expected = 42;
        let actual = mock_output.next_elem()?;
        Ok(assert!(expected == actual))
    }
}