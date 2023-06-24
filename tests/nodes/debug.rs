#[cfg(test)]
mod nodes {

    use std::sync::mpsc::channel;

    use fbp::job::Job;

    use fbp::debug::DebugNode;

    #[test]
    fn should_debug() {
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let mut debug = DebugNode {
            name: "DebugNode".into(),
            input: r1,
            output: s2,
        };
        s1.send(1);
        s1.send(2);
        s1.send(3);
        debug.handle();
        debug.handle();
        debug.handle();
    }
}
