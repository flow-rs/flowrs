use crate::{
    sched::flow::Flow,
    node::{ChangeObserver},
    scheduler::{Scheduler, SchedulingInfo},
};
use std::{
    fmt,
    sync::{Arc, mpsc::Sender, Mutex},
};
use threadpool::ThreadPool;
use anyhow::{Context as AnyhowContext, Result};

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

/// A Send + Sync thread pool.
#[derive(Debug, Clone)]
pub struct SyncThreadPool {
    pool: Arc<Mutex<ThreadPool>>,
}

// From https://github.com/rust-threadpool/rust-threadpool/issues/96
impl SyncThreadPool {
    /// Create a new thread pool with the specified size.
    pub fn new(num_threads: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(ThreadPool::new(num_threads))),
        }
    }

    /// Execute a job on the thread pool.
    pub fn execute(&self, job: impl FnOnce() + Send + 'static) {
        self.pool
            .lock()
            .expect("could not lock thread pool mutex")
            .execute(job)
    }
}

pub struct MultiThreadedExecutor {
    thread_pool: SyncThreadPool,
    controller: Arc<Mutex<ExecutionController>>,
    observer: ChangeObserver
}

impl MultiThreadedExecutor {
    pub fn new(num_threads: usize, observer: ChangeObserver) -> Self {   
        Self {
            thread_pool: SyncThreadPool::new(num_threads),
            controller: Arc::new(Mutex::new(ExecutionController::new(observer.notifier.clone()))),
            observer: observer
        }
    }

    fn run_update_loop<S>(&mut self, flow: &Flow, mut scheduler: S)
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

                if let Some(node) = flow.get_node(node_idx) {
                    self.thread_pool.execute(move || {
                        let _ = node.lock().unwrap().update();
                    });
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
    }

}

impl Executor for MultiThreadedExecutor {
    fn run<S>(&mut self, flow: Flow, scheduler: S) -> Result<()> 
    where
        S: Scheduler + std::marker::Send,
    {
        flow.init_all().context(format!("Unable to init all nodes."))?; 

        flow.ready_all().context(format!("Unable to make all nodes ready."))?;
    
        self.run_update_loop(&flow, scheduler);

        flow.shutdown_all().context(format!("Unable to shutdown all nodes"))?;

        Ok(())
    }

    fn controller(&self) -> Arc<Mutex<ExecutionController>> {
        self.controller.clone()
    }
}
