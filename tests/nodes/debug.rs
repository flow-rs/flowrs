#[cfg(test)]
mod nodes {
    use flow::{
        connection::{connect, ConnectError, Edge},
        debug::DebugNode,
        node::{Context, Node, State},
    };
    use serde_json::Value;

    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = State::new(Context::new());
        let mock_output = Edge::new();
        let fst = DebugNode::new("AddNodeI32", context.clone(), Value::Null);
        let snd = DebugNode::new("AddNodeI32", context, Value::Null);
        connect(fst.output.clone(), snd.input.clone());
        connect(snd.output.clone(), mock_output.clone());
        let _ = fst.input.send(1);
        let _ = fst.update();
        let _ = snd.update();

        let expected = 1;
        let actual = mock_output.next_elem()?;
        Ok(assert!(expected == actual))
    }
}
