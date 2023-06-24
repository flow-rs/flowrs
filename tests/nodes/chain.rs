#[cfg(test)]
mod nodes {
    use fbp::job::Job;
    use std::{sync::mpsc::channel};

    use fbp::add::AddNode;

    /// Test scenario:
    /// 
    /// Output: r7
    /// Inputs: s1-s4
    /// 
    ///       r7
    ///       |
    ///      add3
    ///    /     \
    ///  add1   add2
    ///  /  \   /  \
    /// s1  s2 s3  s4
    #[test]
    fn should_add_132() {
        // All 7 edges of the graph
        let (s1, r1) = channel();
        let (s2, r2) = channel();
        let (s3, r3) = channel();
        let (s4, r4) = channel();
        let (s5, r5) = channel();
        let (s6, r6) = channel();
        let (s7, r7) = channel();
        // Nodes
        let mut add1 = AddNode {
            state: None,
            name: "AddNodeI32".to_owned(),
            input: vec![r1, r2],
            output: vec![s5],
            neutral: 0,
            to_state: |i| i,
            from_state: |i| i,
        };
        let mut add2 = AddNode {
            state: None,
            name: "AddNodeI32".to_owned(),
            input: vec![r3, r4],
            output: vec![s6],
            neutral: 0,
            to_state: |i| i,
            from_state: |i| i,
        };
        let mut add3 = AddNode {
            state: None,
            name: "AddNodeI32".to_owned(),
            input: vec![r5, r6],
            output: vec![s7],
            neutral: 0,
            to_state: |i| i,
            from_state: |i| i,
        };
        // Init queues
        s1.send(1);
        s2.send(2);
        s3.send(3);
        s4.send(4);
        // Mocking a FIFO Queue considering two steps per addition and a buffer of
        // three for scheduling cycles where no item was present for processing.
        let mut jobs = vec![add1, add2, add3];
        println!("Jobs");
        for i in 0..8 {
            println!("Job: {}", i%3);
            jobs[i%3].handle()
        }

        assert!(r7.recv().unwrap() == 10);
    }
}