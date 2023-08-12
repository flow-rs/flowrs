use anyhow::Error;
use flowrs::connection::{Input, Output};
use flowrs::node::{ChangeObserver, InitError, Node, UpdateError};
use flowrs_derive::RuntimeConnectable;

use std::fs::File;

#[derive(RuntimeConnectable)]
pub struct DummyNode {
    pub input_1: Input<i32>,
    pub output_1: Output<i32>,
    err_on_init: bool,
}

impl DummyNode {
    pub fn new(change_observer: Option<&ChangeObserver>, err_on_init: bool) -> Self {
        Self {
            input_1: Input::new(),
            output_1: Output::new(change_observer),
            err_on_init: err_on_init,
        }
    }
}

impl Node for DummyNode {
    fn on_init(&self) -> Result<(), InitError> {
        if self.err_on_init {
            let _file = File::open("").map_err(|err| InitError::Other(err.into()))?;
        }
        Ok(())
    }
}

#[derive(RuntimeConnectable)]
pub struct ErrNode<T> {
    pub input_1: Input<T>,
    pub output_1: Output<T>,
}

impl<T: Send> ErrNode<T> {
    pub fn new(change_observer: Option<&ChangeObserver>) -> Self {
        Self {
            input_1: Input::new(),
            output_1: Output::new(change_observer),
        }
    }
}

impl<T: Send> Node for ErrNode<T> {
    fn on_update(&mut self) -> Result<(), UpdateError> {
        Err(UpdateError::Other(Error::msg(
            "not feeling like being a node...",
        )))
    }
}

#[cfg(test)]
mod test_execution {

    use anyhow::Error;
    use flowrs::connection::{connect, Input};
    use flowrs::exec::execution::{Executor, StandardExecutor};
    use flowrs::exec::node_updater::{MultiThreadedNodeUpdater, SingleThreadedNodeUpdater};
    use flowrs::flow_impl::Flow;
    use flowrs::node::ChangeObserver;
    use flowrs::nodes::node_description::NodeDescription;
    use flowrs::sched::round_robin::RoundRobinScheduler;
    use flowrs::version::Version;

    use crate::sched::test_execution::{DummyNode, ErrNode};

    use std::collections::HashMap;
    use std::{sync::mpsc, thread, time::Duration};

    #[test]
    fn test_executor() -> Result<(), Error> {
        let (sender, receiver) = mpsc::channel();
        let change_observer: ChangeObserver = ChangeObserver::new();
        let n1: DummyNode = DummyNode::new(Some(&change_observer), false);
        let mock_input = Input::<i32>::new();
        connect(n1.output_1.clone(), mock_input.clone());
        let mut flow = Flow::new("flow_1", Version::new(1, 0, 0), HashMap::new());
        n1.input_1.send(1)?;
        flow.add_node(n1);
        let thread_handle = thread::spawn(move || {
            let num_threads = 4;
            let mut executor = StandardExecutor::new(change_observer);
            let node_updater = MultiThreadedNodeUpdater::new(num_threads);
            let scheduler = RoundRobinScheduler::new();
            let _ = sender.send(executor.controller());
            let _ = executor.run(flow, scheduler, node_updater);
        });
        let controller = receiver.recv().unwrap();
        thread::sleep(Duration::from_secs(3));
        println!("CANCEL");
        controller.lock().unwrap().cancel();
        thread_handle.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_error_behavior() {
        let change_observer: ChangeObserver = ChangeObserver::new();

        let n1: DummyNode = DummyNode::new(Some(&change_observer), true);
        let n2: DummyNode = DummyNode::new(Some(&change_observer), true);
        let mut flow = Flow::new_empty("flow_1", Version::new(1, 0, 0));

        flow.add_node(n1);
        flow.add_node(n2);

        let mut ex = StandardExecutor::new(change_observer);

        match ex.run(
            flow,
            RoundRobinScheduler::new(),
            MultiThreadedNodeUpdater::new(1),
        ) {
            Ok(_) => todo!(),
            Err(err) => eprintln!("Error: {:?}", err),
        }
    }

    #[test]
    fn should_fail_on_async_update() -> Result<(), Error> {
        let (sender, receiver) = mpsc::channel();
        let change_observer: ChangeObserver = ChangeObserver::new();
        let n1 = ErrNode::new(Some(&change_observer));
        let n2 = ErrNode::new(Some(&change_observer));
        let mock_input = Input::<i32>::new();
        connect(n1.output_1.clone(), mock_input.clone());
        let mut flow = Flow::new_empty("flow_1", Version::new(1, 0, 0));
        n1.input_1.send(1)?;
        n2.input_1.send(true)?;
        flow.add_node(n1);
        flow.add_node_with_id_and_desc(
            n2,
            5,
            NodeDescription {
                name: "Sad Node".into(),
                description: "Not doing much".into(),
                kind: "ErrNode".into(),
            },
        );
        let thread_handle = thread::spawn(move || {
            let num_threads = 2;
            let mut executor = StandardExecutor::new(change_observer);
            let node_updater = MultiThreadedNodeUpdater::new(num_threads);
            let scheduler = RoundRobinScheduler::new();
            let _ = sender.send(executor.controller());
            let errs = executor.run(flow, scheduler, node_updater);
            assert!(errs.is_err());
            if let Err(e) = errs {
                let fst_err = r#"NodeUpdateError { source: Other(not feeling like being a node...), node_id: Some(1), node_desc: Some(NodeDescription { name: "", description: "", kind: "" }) }"#;
                let snd_err = r#"NodeUpdateError { source: Other(not feeling like being a node...), node_id: Some(5), node_desc: Some(NodeDescription { name: "Sad Node", description: "Not doing much", kind: "ErrNode" }) }"#;
                let fst_cond = e.to_string()
                    == format!(
                        "Errors occured while updating nodes: [{}, {}]",
                        fst_err, snd_err
                    );
                let snd_cond = e.to_string()
                    == format!(
                        "Errors occured while updating nodes: [{}, {}]",
                        fst_err, snd_err
                    );
                assert!(fst_cond || snd_cond);
            }
        });
        let controller = receiver.recv().unwrap();
        thread::sleep(Duration::from_millis(100));
        println!("CANCEL");
        controller.lock().unwrap().cancel();
        thread_handle.join().unwrap();
        Ok(())
    }

    #[test]
    fn should_fail_on_sync_update() -> Result<(), Error> {
        let (sender, receiver) = mpsc::channel();
        let change_observer: ChangeObserver = ChangeObserver::new();
        let n1 = ErrNode::new(Some(&change_observer));
        let n2 = ErrNode::new(Some(&change_observer));
        let mock_input = Input::<i32>::new();
        connect(n1.output_1.clone(), mock_input.clone());
        let mut flow = Flow::new_empty("flow_1", Version::new(1, 0, 0));
        n1.input_1.send(1)?;
        n2.input_1.send(true)?;
        flow.add_node(n1);
        flow.add_node_with_id_and_desc(
            n2,
            5,
            NodeDescription {
                name: "Sad Node".into(),
                description: "Not doing much".into(),
                kind: "ErrNode".into(),
            },
        );
        let thread_handle = thread::spawn(move || {
            let mut executor = StandardExecutor::new(change_observer);
            let node_updater = SingleThreadedNodeUpdater::new(None);
            let scheduler = RoundRobinScheduler::new();
            let _ = sender.send(executor.controller());
            let errs = executor.run(flow, scheduler, node_updater);
            assert!(errs.is_err());
            if let Err(e) = errs {
                let fst_err = r#"NodeUpdateError { source: Other(not feeling like being a node...), node_id: Some(1), node_desc: Some(NodeDescription { name: "", description: "", kind: "" }) }"#;
                let snd_err = r#"NodeUpdateError { source: Other(not feeling like being a node...), node_id: Some(5), node_desc: Some(NodeDescription { name: "Sad Node", description: "Not doing much", kind: "ErrNode" }) }"#;
                let fst_cond = e.to_string()
                    == format!(
                        "Errors occured while updating nodes: [{}, {}]",
                        fst_err, snd_err
                    );
                let snd_cond = e.to_string()
                    == format!(
                        "Errors occured while updating nodes: [{}, {}]",
                        fst_err, snd_err
                    );
                assert!(fst_cond || snd_cond);
            }
        });
        let controller = receiver.recv().unwrap();
        thread::sleep(Duration::from_millis(100));
        println!("CANCEL");
        controller.lock().unwrap().cancel();
        thread_handle.join().unwrap();
        Ok(())
    }
}
