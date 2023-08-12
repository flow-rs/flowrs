use crate::{
    exec::{
        execution_controller::ExecutionController,
        execution_state::ExecutionState,
        node_updater::{NodeUpdateError, NodeUpdater, SleepMode},
    },
    flow::flow::Flow,
    node::ChangeObserver,
    scheduler::{Scheduler, SchedulingInfo},
};

use anyhow::{Context as AnyhowContext, Result};
use std::sync::{Arc, Mutex};
use thiserror::Error;

pub trait Executor {
    fn run<S, U>(&mut self, flow: Flow, scheduler: S, node_updater: U) -> Result<()>
    where
        S: Scheduler + std::marker::Send,
        U: NodeUpdater + Drop;

    fn controller(&self) -> Arc<Mutex<ExecutionController>>;
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Errors occured while updating nodes: {errors:?}")]
    UpdateErrorCollection { errors: Vec<NodeUpdateError> },
}

pub struct StandardExecutor {
    controller: Arc<Mutex<ExecutionController>>,
    observer: ChangeObserver,
}

impl StandardExecutor {
    pub fn new(observer: ChangeObserver) -> Self {
        Self {
            controller: Arc::new(Mutex::new(ExecutionController::new(
                observer.notifier.clone(),
            ))),
            observer,
        }
    }

    fn run_update_loop<S, U>(
        &mut self,
        flow: &Flow,
        mut scheduler: S,
        mut node_updater: U,
    ) -> Result<(), ExecutionError>
    where
        S: Scheduler,
        U: NodeUpdater,
    {
        self.controller
            .lock()
            .unwrap()
            .set_state(ExecutionState::Running);

        let mut info = SchedulingInfo::new(flow.num_nodes());

        let update_controllers = flow.get_update_controllers();

        while !self.controller.lock().unwrap().cancellation_requested() {
            // Run an epoch (an update of each node).
            scheduler.restart_epoch(&mut info);

            //println!("                                                                                                    {:?} NEW EPOCH", std::thread::current().id());

            while !scheduler.epoch_is_over(&mut info) {
                let node_idx = scheduler.get_next_node_idx();
                //println!("                                                                                                    {:?} {}", std::thread::current().id(), node_idx);

                let node = flow.node_by_index(node_idx);
                if let Some(n) = node {
                    node_updater.update(n.clone());
                }
            }

            // Sleep if necessary.
            match node_updater.sleep_mode() {
                SleepMode::None => {}

                SleepMode::Reactive => {
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

                SleepMode::FixedFrequency(fps) => {
                    
                }
            }

            // Check if async errors occured.
            let errors: Vec<NodeUpdateError> = node_updater.errors().into_iter().map(|mut err| {
                if let Some(id) = err.node_id {
                    err.node_id = Some(id);
                    if let Some(desc) = flow.node_description_by_id(id) {
                        err.node_desc = Some(desc.clone());
                    }
                }
                err
            }).collect();
            if !errors.is_empty() {
                return Err(ExecutionError::UpdateErrorCollection { errors });
            }
        }

        // Cancel long-running node updates.
        update_controllers
            .iter()
            .for_each(|uc| uc.lock().unwrap().cancel());

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
    fn run<S, U>(&mut self, flow: Flow, scheduler: S, node_updater: U) -> Result<(), anyhow::Error>
    where
        S: Scheduler + std::marker::Send,
        U: NodeUpdater + Drop,
    {
        //TODO: Fix error flow.

        flow.init_all()
            .context(format!("Unable to init all nodes."))?;

        flow.ready_all()
            .context(format!("Unable to make all nodes ready."))?;

        self.run_update_loop(&flow, scheduler, node_updater)?;

        flow.shutdown_all()
            .context(format!("Unable to shutdown all nodes"))?;

        Ok(())
    }

    fn controller(&self) -> Arc<Mutex<ExecutionController>> {
        self.controller.clone()
    }
}
