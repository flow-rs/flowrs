#[cfg(test)]
mod nodes {
    use flow::job::Job;
    use flow::job::Connectable;
    use std::sync::mpsc::{channel, Receiver, Sender};

    use flow::add::AddNode;

    /// Test scenario:
    ///
    /// Output: mock_r
    /// Inputs: s1-s4
    ///
    ///     mock_r
    ///       |
    ///      add3
    ///    /     \
    ///  add1   add2
    ///  /  \   /  \
    /// s1  s2 s3  s4
    #[test]
    fn should_chain_homogenious_nodes() {
        // All 7 edges of the graph
        let (mock_s, mock_r): (Sender<i32>, Receiver<i32>) = channel();

        let mut add1 = AddNode::new("Add1", 0, |i| i, |i| i);
        let mut add2 = AddNode::new("Add2", 0, |i| i, |i| i);
        let mut add3 = AddNode::new("Add3", 0, |i| i, |i| i);
        // Init queues
        let _ = add1.input()[0].send(1);
        let _ = add1.input()[1].send(2);
        let _ = add2.input()[0].send(3);
        let _ = add2.input()[1].send(4);
        add1.connect(vec![add3.input()[0].clone()]);
        add2.connect(vec![add3.input()[1].clone()]);
        add3.connect(vec![mock_s]);
        // Mocking a FIFO Queue considering two steps per addition and a buffer of
        // three for scheduling cycles where no item was present for processing.
        let mut jobs = vec![add1, add2, add3];
        println!("Jobs");
        for i in 0..9 {
            println!("Job: {}", i % 3);
            jobs[i % 3].handle()
        }

        assert!(mock_r.recv().unwrap() == 10);
    }
    #[test]
    fn should_connect_homogenious_nodes() {}
}
