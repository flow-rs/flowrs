use crate::{
    node::{ChangeObserver, UpdateError},
    sched::flow::Flow,
    scheduler::{Scheduler, SchedulingInfo},
};
use anyhow::{Context as AnyhowContext, Result};
use std::{
    fmt,
    sync::{mpsc::Sender, Arc, Mutex}, thread::{self, JoinHandle},
};

#[derive(PartialEq, Clone, Copy)]
pub enum ExecutorState {
    Ready,
    Sleeping,
    Running,
}

pub struct ExecutionController {
    state: ExecutorState,
    cancellation_requested: bool,
    change_notifier: Sender<bool>,
}

impl fmt::Display for ExecutorState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ExecutorState::Ready => write!(f, "Ready"),
            ExecutorState::Sleeping => write!(f, "Sleeping"),
            ExecutorState::Running => write!(f, "Running"),
        }
    }
}

impl ExecutionController {
    pub fn new(change_notifier: Sender<bool>) -> Self {
        Self {
            state: ExecutorState::Ready,
            cancellation_requested: false,
            change_notifier: change_notifier,
        }
    }

    pub fn cancel(&mut self) {
        self.cancellation_requested = true;
        if self.state == ExecutorState::Sleeping {
            self.change_notifier.send(true);
        }
    }

    pub fn state(&self) -> ExecutorState {
        self.state
    }

    fn set_state(&mut self, s: ExecutorState) {
        self.state = s
    }

    fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }
}

pub trait Executor {
    fn run<S>(&mut self, flow: Flow, scheduler: S) -> Result<()>
    where
        S: Scheduler + std::marker::Send;

    fn controller(&self) -> Arc<Mutex<ExecutionController>>;
}

pub struct MultiThreadedExecutor {
    controller: Arc<Mutex<ExecutionController>>,
    observer: ChangeObserver,
    num_threads: usize
}

impl MultiThreadedExecutor {
    pub fn new(num_threads: usize, observer: ChangeObserver) -> Self {
        Self {
            num_threads,
            controller: Arc::new(Mutex::new(ExecutionController::new(
                observer.notifier.clone(),
            ))),
            observer,
        }
    }

    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    fn run_update_loop<S>(&mut self, flow: &Flow, mut scheduler: S) -> Result<(), anyhow::Error>
    where
        S: Scheduler,
    {
        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutorState::Running);

        let info = SchedulingInfo {
            num_nodes: flow.num_nodes(),
            priorities: Vec::new(),
        };

        while !self.controller.lock().unwrap().cancellation_requested() {
            //println!("{:?} SCHEDULE EPOCH", std::thread::current().id());

            scheduler.restart_epoch();

            while !scheduler.epoch_is_over(&info) {
                let node_idx = scheduler.get_next_node_idx(&info);

                let mut chunks = vec![flow.nodes.clone().into_boxed_slice()];
                // Using log2 to keep the recursive splitting aligned with the amount of threads.
                for _ in 0..self.num_threads().ilog2() {
                    for chunk in &mut chunks.clone() {
                        let mid = chunk.len() / 2;
                        let (lhs, rhs) = chunk.split_at(mid);
                        chunks = vec![
                            lhs.to_vec().into_boxed_slice(),
                            rhs.to_vec().into_boxed_slice(),
                        ]
                    }
                }
                let handlers: Vec<JoinHandle<()>> = chunks
                    .into_iter()
                    .map(|chunk| {
                        thread::spawn(move || {
                            for node in chunk.into_iter() {
                                node.lock().unwrap().update();
                            }
                        })
                    })
                    .collect();
                for handle in handlers {
                    match handle.join() {
                        Ok(_) => (),
                        Err(_) => return Err(anyhow::Error::msg("At least one node failed")),
                    };
                }
            }

            //println!("{:?} WAIT FOR CHANGES", std::thread::current().id());
            self.controller
                .lock()
                .unwrap()
                .set_state(ExecutorState::Sleeping);

            self.observer.wait_for_changes();

            //println!("{:?} WAIT FOR CHANGES DONE", std::thread::current().id());
            self.controller
                .lock()
                .unwrap()
                .set_state(ExecutorState::Running);
        }

        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutorState::Ready);
        Ok(())
    }
}

impl Executor for MultiThreadedExecutor {
    fn run<S>(&mut self, flow: Flow, scheduler: S) -> Result<(), anyhow::Error>
    where
        S: Scheduler + std::marker::Send,
    {
        flow.init_all()
            .context(format!("Unable to init all nodes."))?;

        flow.ready_all()
            .context(format!("Unable to make all nodes ready."))?;

        self.run_update_loop(&flow, scheduler)?;

        flow.shutdown_all()
            .context(format!("Unable to shutdown all nodes"))?;

        Ok(())
    }

    fn controller(&self) -> Arc<Mutex<ExecutionController>> {
        self.controller.clone()
    }
}
