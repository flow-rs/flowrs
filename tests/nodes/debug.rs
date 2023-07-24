#[cfg(test)]
mod nodes {

    use std::{sync::Arc, borrow::BorrowMut};

    use flow::{connection::{ConnectError, Edge, connect}, job::Context, debug::DebugNode, Node};
    use serde_json::Value;

    #[test]
    fn should_add_132() -> Result<(), ConnectError<i32>> {
        let context = Arc::new(Context {});
        let mock_output = Edge::new();
        let mut fst = DebugNode::new("AddNodeI32", context.clone(), Value::Null);
        let mut snd = DebugNode::new("AddNodeI32", context, Value::Null);
        connect(fst.output.clone(), snd.input.clone());
        connect(snd.output.clone(), mock_output.clone());
        let _ = fst.input.send(1);
        fst.update();
        snd.update();

        let expected = 1;
        let actual = mock_output.next_elem()?;
        Ok(assert!(expected == actual))
    }
}
