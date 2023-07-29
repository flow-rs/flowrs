#[cfg(test)]
mod nodes {
    use flow::app_state::FlowType;
    use flow::connection::ConnectError;
    use flow::job::Connectable;
    use flow::job::Context;
    use flow::job::Job;
    use std::rc::Rc;
    use std::sync::Mutex;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::sync::Arc;

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
    fn should_chain_homogenious_nodes() -> Result<(), ConnectError<i32>> {
        // All 7 edges of the graph
        let (mock_s, mock_r): (Sender<i32>, Receiver<i32>) = channel();

        let context = Arc::new(Context {});
        let add1 = AddNode::new("Add1", context.clone());
        let add2 = AddNode::new("Add2", context.clone());
        let add3 = AddNode::new("Add3", context.clone());
        // Init queues
        let _ = add1.send_at(0, 1);
        let _ = add1.send_at(1, 2);
        let _ = add2.send_at(0, 3);
        let _ = add2.send_at(1, 4);
        add1.chain(vec![add3.input_at(0)?]);
        add2.chain(vec![add3.input_at(1)?]);
        add3.chain(vec![mock_s]);
        // Mocking a FIFO Queue considering two steps per addition and a buffer of
        // three for scheduling cycles where no item was present for processing.
        let mut jobs = vec![add1, add2, add3];
        println!("Jobs");
        for i in 0..9 {
            println!("Job: {}", i % 3);
            jobs[i % 3].on_handle()
        }

        assert!(mock_r.recv()? == 10);
        Ok(())
    }

    #[test]
    fn should_chain_in_heap() -> Result<(), ConnectError<FlowType>> {
        // All 7 edges of the graph
        let (mock_s, mock_r): (Sender<FlowType>, Receiver<FlowType>) = channel();

        let context = Arc::new(Context {});
        let add1 = Arc::new(Mutex::new(AddNode::new("Add1", context.clone())));
        let add2 = Arc::new(Mutex::new(AddNode::new("Add2", context.clone())));
        let add3 = Arc::new(Mutex::new(AddNode::new("Add3", context.clone())));
        // Init queues
        let jobs = vec![add1, add2, add3];
        let _ = jobs[0].lock().unwrap().send_at(0, FlowType(Rc::new(1)));
        let _ = jobs[0].lock().unwrap().send_at(1, FlowType(Rc::new(2)));
        let _ = jobs[1].lock().unwrap().send_at(0, FlowType(Rc::new(3)));
        let _ = jobs[1].lock().unwrap().send_at(1, FlowType(Rc::new(4)));
        jobs[0].lock().unwrap().chain(vec![jobs[2].lock().unwrap().input_at(0)?]);
        jobs[1].lock().unwrap().chain(vec![jobs[2].lock().unwrap().input_at(1)?]);
        jobs[2].lock().unwrap().chain(vec![mock_s]);
        // Mocking a FIFO Queue considering two steps per addition and a buffer of
        // three for scheduling cycles where no item was present for processing.
        println!("Jobs");
        for i in 0..9 {
            println!("Job: {}", i % 3);
            jobs[i % 3].lock().unwrap().on_handle()
        }

        assert!(*mock_r.recv()?.0.downcast::<i32>().unwrap() == 10);
        Ok(())
    }
}
