use crate::{
    sched::flow::Flow,
    node::{ChangeObserver, Context, State},
    scheduler::{Scheduler, SchedulingInfo},
};
use std::{
    fmt,
    sync::{Arc, Condvar, Mutex},
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
    condition: Arc<(Mutex<bool>, Condvar)>,
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
    pub fn new(condition: Arc<(Mutex<bool>, Condvar)>) -> Self {
        Self {
            state: ExecutorState::Ready,
            cancellation_requested: false,
            condition: condition,
        }
    }

    pub fn cancel(&mut self) {
        self.cancellation_requested = true;
        if self.state == ExecutorState::Sleeping {
            self.wakeup();
        }
    }

    pub fn state(&self) -> ExecutorState {
        self.state
    }

    fn set_state(&mut self, s: ExecutorState) {
        println!("{}", s);
        self.state = s
    }

    fn cancellation_requested(&self) -> bool {
        self.cancellation_requested
    }

    fn wakeup(&mut self) {
        let (lock, cvar) = &*self.condition;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    }
}

struct ExecutionHibernator {
    num_epochs_to_do: i32,
    condition: Arc<(Mutex<bool>, Condvar)>,
}

impl ExecutionHibernator {
    pub fn new(condition: Arc<(Mutex<bool>, Condvar)>) -> Self {
        Self {
            num_epochs_to_do: 0,
            condition: condition,
        }
    }

    fn sleep_if_possible(&mut self, controller: Arc<Mutex<ExecutionController>>) {
        self.num_epochs_to_do = 0.max(self.num_epochs_to_do - 1);

        if self.num_epochs_to_do != 0 {
            return;
        }

        controller
            .lock()
            .unwrap()
            .set_state(ExecutorState::Sleeping);
        self.sleep();
        controller.lock().unwrap().set_state(ExecutorState::Running);
    }

    fn sleep(&mut self) {
        let (lock, cvar) = &*self.condition;
        let mut ready = lock.lock().unwrap();
        while !*ready {
            ready = cvar.wait(ready).unwrap();
        }
    }

    fn wakeup(&mut self) {
        let (lock, cvar) = &*self.condition;
        let mut ready = lock.lock().unwrap();
        *ready = true;
        cvar.notify_one();
    }
}

impl ChangeObserver for ExecutionHibernator {
    fn on_change(&mut self) {
        self.num_epochs_to_do = 1; // For now just a single epoch per change.
        self.wakeup();
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
    hibernator: Arc<Mutex<ExecutionHibernator>>,
}

impl MultiThreadedExecutor {
    pub fn new(num_threads: usize, context: State<Context>) -> Self {
        let condition = Arc::new((Mutex::new(false), Condvar::new()));

        let res = Self {
            thread_pool: SyncThreadPool::new(num_threads),
            controller: Arc::new(Mutex::new(ExecutionController::new(condition.clone()))),
            hibernator: Arc::new(Mutex::new(ExecutionHibernator::new(condition.clone()))),
        };

        context
            .0
            .lock()
            .unwrap()
            .set_observer(res.hibernator.clone());

        res
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
            scheduler.restart_epoch();

            while !scheduler.epoch_is_over(&info) {
                let node_idx = scheduler.get_next_node_idx(&info);

                if let Some(node) = flow.get_node(node_idx) {
                    self.thread_pool.execute(move || {
                        let _ = node.lock().unwrap().update();
                    });
                }
            }

            self.hibernator
                .lock()
                .unwrap()
                .sleep_if_possible(self.controller.clone());
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
