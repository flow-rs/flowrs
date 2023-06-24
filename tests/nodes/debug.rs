#[cfg(test)]
mod nodes {

    use fbp::job::Job;

    use fbp::debug::DebugNode;

    #[test]
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
