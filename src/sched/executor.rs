use crate::{
    node::{ChangeObserver, UpdateError, self},
    sched::flow::Flow,
    scheduler::{Scheduler, SchedulingInfo},
    connection::RuntimeNode
};

use thiserror::Error;
use crossbeam_channel::{unbounded, Sender, Receiver};
use anyhow::{Context as AnyhowContext, Result};
use std::{
    fmt,
    sync::{ Arc, Mutex}, thread::{self, JoinHandle},
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
    change_notifier: std::sync::mpsc::Sender<bool>,
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
    pub fn new(change_notifier: std::sync::mpsc::Sender<bool>) -> Self {
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

enum WorkerCommand {
    Update(Arc<Mutex<dyn RuntimeNode + Send>>),
    Cancel
}

pub struct NodeUpdater {
    num_workers: usize,
    workers: Vec<JoinHandle<Result<(), UpdateError>>>,

    command_channel: (Sender<WorkerCommand>, Receiver<WorkerCommand>),
    error_channel: (Sender<UpdateError>, Receiver<UpdateError>)
}

impl NodeUpdater {

    fn new(num_workers: usize) -> Self {
        let mut obj = Self {
            num_workers: num_workers,
            workers: Vec::new(),
            
            command_channel: unbounded(),
            error_channel:  unbounded()
        };

        obj.create_workers();
        obj
    }


    fn update(&mut self, node: Arc<Mutex<dyn RuntimeNode + Send>>) -> Result<(), UpdateError> {

        if self.num_workers == 0 { // single-threaded
            return node.lock().unwrap().on_update();

        } else { // multi-threaded 
            if let Err(err) = self.command_channel.0.send(WorkerCommand::Update(node.clone())) {
                Result::Err(UpdateError::Other(err.into()))
            } else {
                Ok(())
            }
        }
    }

    fn errors(&mut self) -> Vec<UpdateError> {
       let errors: Vec<UpdateError> = self.error_channel.1.try_iter().collect();
       errors
    }

    fn destroy_workers(&mut self) {
        for _ in 0..self.num_workers {
            let _res = self.command_channel.0.send(WorkerCommand::Cancel);
        }

        for worker in self.workers.drain(..) {
            let _res = worker.join();
        }
    }

    fn create_workers(&mut self){

        for _ in 0..self.num_workers {

            let update_receiver_clone = self.command_channel.1.clone();
            let error_sender_clone = self.error_channel.0.clone();

            let thread_handle: thread::JoinHandle<Result<(), UpdateError>> = thread::spawn( move || -> Result<(), UpdateError>  {
                
                loop {

                    let update_receiver_res = update_receiver_clone.recv();
                    match update_receiver_res {
                        
                        Result::Err(err) => {
                            println!("{:?} THREAD UPDATE ERROR {:?}", std::thread::current().id(), err);
                            let _res = error_sender_clone.send(UpdateError::Other(err.into()));
                            break Ok(());
                        }
                        
                        Result::Ok(command) => {
                            
                            match command {
                            
                                WorkerCommand::Cancel => {
                                    break Ok(());
                                },
                            
                                WorkerCommand::Update(node) => {
                                    //let name = node.lock().unwrap().name().to_string();
                                    //println!("{:?} THREAD UPDATE {}", std::thread::current().id(), name);
                                    
                                    if let Ok(n) = node.try_lock() {
                                        if let Err(err) = n.on_update() {
                                            let _res = error_sender_clone.send(err);
                                            break Ok(());
                                        }
                                    } else {
                                        //println!("{:?} THREAD UPDATE {} - DIDN'T GET LOCK", std::thread::current().id(), name);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            self.workers.push(thread_handle);
        }
    }

}

impl Drop for NodeUpdater {
    fn drop(&mut self) {
        self.destroy_workers();
    }
}

#[derive(Error, Debug)]
pub enum ExecutionError {

    #[error("Errors occured while updating nodes.")]
    UpdateErrorCollection {
        errors: Vec<UpdateError>
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error)
} 

pub struct StandardExecutor {
    controller: Arc<Mutex<ExecutionController>>,
    observer: ChangeObserver,
    num_workers: usize
}

impl StandardExecutor {
    pub fn new(num_workers: usize, observer: ChangeObserver) -> Self {
        Self {
            num_workers,
            controller: Arc::new(Mutex::new(ExecutionController::new(
                observer.notifier.clone(),
            ))),
            observer
        }
    }

    pub fn num_workers(&self) -> usize {
        self.num_workers
    }

    fn run_update_loop<S>(&mut self, flow: &Flow, mut scheduler: S) -> Result<(), ExecutionError>
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

        let mut node_updater = NodeUpdater::new(self.num_workers); 

        let update_controllers = flow.get_update_controllers();

        while !self.controller.lock().unwrap().cancellation_requested() {

            // Run an epoch (an update of each node).
            scheduler.restart_epoch();

            println!("                                                                                                    {:?} NEW EPOCH", std::thread::current().id());

            while !scheduler.epoch_is_over(&info) {
                let node_idx = scheduler.get_next_node_idx(&info);
                println!("                                                                                                    {:?} {}", std::thread::current().id(), node_idx);

                let node = flow.get_node(node_idx);
                if let Some(n) = node {
                    if let Err(err) = node_updater.update(n) {
                        return Err( ExecutionError::UpdateErrorCollection{ errors: vec![err]});
                    }
                }
            }

            // Check if async errors occured.
            let errors = node_updater.errors();
            if !errors.is_empty() {
                return Err( ExecutionError::UpdateErrorCollection { errors: errors });
            }
             

            // Sleep if necessary 
            if (self.num_workers > 0) { // Sleep check only if multi-threaded.
                self.controller
                    .lock()
                    .unwrap()
                    .set_state(ExecutorState::Sleeping);

                self.observer.wait_for_changes();

                self.controller
                    .lock()
                    .unwrap()
                    .set_state(ExecutorState::Running);
            }
        }

        // Cancel long-running node updates.
        update_controllers.iter().for_each(|uc| uc.lock().unwrap().cancel());

        // Drop node updater which destroys all workers. 
        drop(node_updater);

        // All done.
        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutorState::Ready);

        Ok(())
    }
}

impl Executor for StandardExecutor {
    fn run<S>(&mut self, flow: Flow, scheduler: S) -> Result<(), anyhow::Error>
    where
        S: Scheduler + std::marker::Send,
    {

        //TODO: Fix error flow. 

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
