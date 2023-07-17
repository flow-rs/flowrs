#[cfg(test)]
mod nodes {

    use std::sync::Arc;
    use flow::job::Context;
    use flow::job::Job;

    use flow::debug::DebugNode;

    #[test]
    fn should_deserialize_from_json() {
        let context = Arc::new(Context {});
        let json = r#"{"conn": 1, "_context": {}, "name": "Hello Node"}"#;
        let actual: DebugNode<i32, i32> = serde_json::from_str(json).unwrap();
        let expected: DebugNode<i32, i32> = DebugNode::new("Hello Node", context.clone());
        assert!(expected.name() == actual.name());
    }
}