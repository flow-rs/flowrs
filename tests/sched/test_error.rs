use anyhow::Error;
use flowrs::connection::{Input, Output};
use flowrs::node::{ChangeObserver, Node, UpdateError};
use flowrs_derive::RuntimeConnectable;

#[derive(RuntimeConnectable)]
pub struct ErrNode {
    name: String,

    pub input_1: Input<i32>,
    pub output_1: Output<i32>,
    err_on_init: bool,
}

impl ErrNode {
    pub fn new(name: &str, change_observer: Option<&ChangeObserver>, err_on_init: bool) -> Self {
        Self {
            name: name.into(),
            input_1: Input::new(),
            output_1: Output::new(change_observer),
            err_on_init: err_on_init,
        }
    }
}

impl Node for ErrNode {
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
    use flowrs::{
        execution::{Executor, StandardExecutor},
        flow::Flow,
        node::ChangeObserver,
        sched::node_updater::MultiThreadedNodeUpdater,
        sched::scheduler::RoundRobinScheduler,
        version::Version,
    };

    use std::collections::HashMap;
    use std::{sync::mpsc, thread, time::Duration};

    use crate::sched::test_error::ErrNode;

    #[test]
    fn test_executor() -> Result<(), Error> {
        let (sender, receiver) = mpsc::channel();
        let change_observer: ChangeObserver = ChangeObserver::new();
        let n1: ErrNode = ErrNode::new("node_1", Some(&change_observer), false);
        let mock_input = Input::<i32>::new();
        connect(n1.output_1.clone(), mock_input.clone());
        let mut flow = Flow::new("flow_1", Version::new(1, 0, 0), HashMap::new());
        n1.input_1.send(1)?;
        flow.add_node(n1, "some".into());
        let thread_handle = thread::spawn(move || {
            let num_threads = 4;
            let mut executor = StandardExecutor::new(change_observer);
            let node_updater = MultiThreadedNodeUpdater::new(num_threads);
            let scheduler = RoundRobinScheduler::new();
            let _ = sender.send(executor.controller());
            let errs = executor.run(flow, scheduler, node_updater);
            match errs {
                Ok(_) => todo!(),
                Err(e) => assert!(e.to_string() == "Errors occured while updating nodes: [Default { node: \"some\", message: \"not feeling like being a node...\" }]"),
            }
        });
        let controller = receiver.recv().unwrap();
        thread::sleep(Duration::from_secs(1));
        println!("CANCEL");
        controller.lock().unwrap().cancel();
        thread_handle.join().unwrap();
        Ok(())
    }
}
