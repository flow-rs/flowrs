#[cfg(test)]
mod nodes {

    use flow::job::Job;
    use flow::job::Connectable;

    use flow::debug::DebugNode;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn should_debug() {
        let mut debug: DebugNode<i32, i32> = DebugNode::new("DebugNode");
        let _ = debug.input()[0].send(1);
        let _ = debug.input()[0].send(2);
        let _ = debug.input()[0].send(3);
        debug.handle();
        debug.handle();
        debug.handle();
    }
}
