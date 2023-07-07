#[cfg(test)]
mod nodes {

    use std::sync::Arc;

    use flow::job::Connectable;
    use flow::job::Context;
    use flow::job::Job;

    use flow::debug::DebugNode;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn should_debug() {
        let context = Arc::new(Context {});
        let mut debug: DebugNode<i32, i32> = DebugNode::new("DebugNode", context);
        let _ = debug.send(1);
        let _ = debug.send(2);
        let _ = debug.send(3);
        debug.handle();
        debug.handle();
        debug.handle();
    }


    #[test]
    fn should_deserialize_from_json() {
        let context = Arc::new(Context {});
        let json = r#"{"conn": 1, "_context": {}, "name": "Hello Node"}"#;
        let actual: DebugNode<i32, i32> = serde_json::from_str(json).unwrap();
        let expected: DebugNode<i32, i32> = DebugNode::new("Hello Node", context.clone());
        assert!(expected.name() == actual.name());
    }

    #[test]
    #[should_panic(expected = "key must be a string")]
    fn should_no_deserialize_from_invalid_json() {
        let json = r#"{"conn": 1, _context: {}, "name": "Hello Node"}"#;
        let _: DebugNode<i32, i32> = serde_json::from_str(json).unwrap();
    }

    #[test]
    #[should_panic(expected = "missing field `_context`")]
    fn should_no_deserialize_from_invalid_node() {
        let json = r#"{"conn": 1, "name": "Hello Node"}"#;
        let _: DebugNode<i32, i32> = serde_json::from_str(json).unwrap();
    }
}
