use crate::{
    node::{ChangeObserver, UpdateError},
    flow::Flow,
    scheduler::{Scheduler, SchedulingInfo},
    sched::{execution_state::ExecutionState, execution_controller::ExecutionController, node_updater::NodeUpdater}
};

use thiserror::Error;
use anyhow::{Context as AnyhowContext, Result};
use std::{
    sync::{ Arc, Mutex}
};

pub trait Executor {
    fn run<S>(&mut self, flow: Flow, scheduler: S) -> Result<()>
    where
        S: Scheduler + std::marker::Send;

    fn controller(&self) -> Arc<Mutex<ExecutionController>>;
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
            .set_state(ExecutionState::Running);

        let info = SchedulingInfo {
            num_nodes: flow.num_nodes(),
            priorities: Vec::new(),
        };

        let mut node_updater = NodeUpdater::new(self.num_workers); 

        let update_controllers = flow.get_update_controllers();

        while !self.controller.lock().unwrap().cancellation_requested() {

            // Run an epoch (an update of each node).
            scheduler.restart_epoch();

            //println!("                                                                                                    {:?} NEW EPOCH", std::thread::current().id());

            while !scheduler.epoch_is_over(&info) {
                let node_idx = scheduler.get_next_node_idx(&info);
                //println!("                                                                                                    {:?} {}", std::thread::current().id(), node_idx);

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
                    .set_state(ExecutionState::Sleeping);

                self.observer.wait_for_changes();

                self.controller
                    .lock()
                    .unwrap()
                    .set_state(ExecutionState::Running);
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
            .set_state(ExecutionState::Ready);

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
