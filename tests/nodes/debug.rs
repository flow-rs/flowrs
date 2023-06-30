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
}
