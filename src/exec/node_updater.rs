use crate::{
    connection::RuntimeNode, flow::flow::NodeId, node::UpdateError,
    nodes::node_description::NodeDescription,
};
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::{
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};
use metrics::{histogram, increment_counter};
use thiserror::Error;
use tracing::{info_span, Instrument, Span, span};
#[cfg(feature = "tracing")]
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Error, Debug)]
pub struct NodeUpdateError {
    pub source: UpdateError,
    pub node_id: Option<NodeId>,
    pub node_desc: Option<NodeDescription>,
}

impl From<UpdateError> for NodeUpdateError {
    fn from(source: UpdateError) -> Self {
        Self {
            source,
            node_id: None,
            node_desc: None,
        }
    }
}

impl std::fmt::Display for NodeUpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NodeUpdateError {}", self.source)
    }
}

pub enum SleepMode {
    Reactive,
    FixedFrequency(u64),
    None,
}

enum WorkerCommand {
    Update((NodeId, Arc<Mutex<dyn RuntimeNode + Send>>), Option<NodeDescription>, Option<tracing::Id>),
    Cancel,
}

pub trait NodeUpdater {
    fn update(&mut self, node: (NodeId, Arc<Mutex<dyn RuntimeNode + Send>>), node_description: Option<NodeDescription>);
    fn errors(&mut self) -> Vec<NodeUpdateError>;

    fn sleep_mode(&self) -> SleepMode;
}

pub struct MultiThreadedNodeUpdater {
    num_workers: usize,
    workers: Vec<JoinHandle<Result<(), NodeUpdateError>>>,

    command_channel: (Sender<WorkerCommand>, Receiver<WorkerCommand>),
    error_channel: (Sender<NodeUpdateError>, Receiver<NodeUpdateError>),
}

impl MultiThreadedNodeUpdater {
    pub fn new(num_workers: usize) -> Self {
        let mut obj = Self {
            num_workers: num_workers,
            workers: Vec::new(),

            command_channel: unbounded(),
            error_channel: unbounded(),
        };

        obj.create_workers();
        obj
    }

    fn destroy_workers(&mut self) {
        for _ in 0..self.num_workers {
            let _res = self.command_channel.0.send(WorkerCommand::Cancel);
        }

        for worker in self.workers.drain(..) {
            let _res = worker.join();
        }
    }

    fn create_workers(&mut self) {
        for _ in 0..self.num_workers {
            let update_receiver_clone = self.command_channel.1.clone();
            let error_sender_clone = self.error_channel.0.clone();

            let thread_handle: thread::JoinHandle<Result<(), NodeUpdateError>> =
                thread::spawn(move || -> Result<(), NodeUpdateError> {
                    loop {
                        let update_receiver_res = update_receiver_clone.recv();
                        match update_receiver_res {
                            Result::Err(err) => {
                                //println!("{:?} THREAD UPDATE ERROR {:?}", std::thread::current().id(), err);
                                let _res = error_sender_clone.send(NodeUpdateError {
                                    source: UpdateError::RecvError {
                                        message: err.to_string(),
                                    },
                                    node_id: None,
                                    node_desc: None,
                                });
                                break Ok(());
                            }

                            Result::Ok(command) => {
                                match command {
                                    WorkerCommand::Cancel => {
                                        break Ok(());
                                    }

                                    WorkerCommand::Update(node, description, parentId) => {
                                        if let Ok(mut n) = node.1.try_lock() {
                                            let result;
                                            {
                                                let node_id = node.0.to_string();
                                                let update_span = if let Some(ref desc) = description {
                                                    info_span!(
                                                        parent: parentId,
                                                        "mt_on_update",
                                                        node.id = node_id.as_str(),
                                                        node.name = desc.name.as_str(),
                                                        "otel.name" = desc.name.as_str(),
                                                        node.kind = desc.kind.as_str(),
                                                        node.description = desc.description.as_str()
                                                    )
                                                } else {
                                                    info_span!(
                                                        parent: parentId,
                                                        "mt_on_update",
                                                        node.id = node_id.as_str()
                                                    )
                                                };
                                                let now = std::time::Instant::now();
                                                result = n.on_update();
                                                let elapsed = now.elapsed();
                                                // publish histogram
                                                if let Some(desc) = description {
                                                    let name = desc.name.clone();
                                                    let kind = desc.kind.clone();
                                                    let description = desc.description.clone();
                                                    histogram!("flowrs.node.update_time", elapsed.as_millis() as f64, "node.id" => node.0.to_string(), "node.name" => name, "node.kind" => kind, "node.description" => description);
                                                } else {
                                                    histogram!("flowrs.node.update_time", elapsed.as_millis() as f64, "node.id" => node.0.to_string());
                                                }
                                            }
                                            if let Err(err) = result {
                                                let _res =
                                                    error_sender_clone.send(NodeUpdateError {
                                                        source: err,
                                                        node_id: Some(node.0),
                                                        node_desc: None,
                                                    });
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

impl NodeUpdater for MultiThreadedNodeUpdater {
    fn update(&mut self, node: (NodeId, Arc<Mutex<dyn RuntimeNode + Send>>), node_description: Option<NodeDescription>) {
        //let cloned_node = node.clone();
        let parentSpan = Span::current();
        let id = parentSpan.id().clone();
        self.command_channel
            .0
            .send(WorkerCommand::Update(node, node_description, id))
            .expect("Unable to write to command channel.");
    }

    fn errors(&mut self) -> Vec<NodeUpdateError> {
        let errors: Vec<NodeUpdateError> = self.error_channel.1.try_iter().collect();
        errors
    }

    fn sleep_mode(&self) -> SleepMode {
        SleepMode::Reactive
    }
}

impl Drop for MultiThreadedNodeUpdater {
    fn drop(&mut self) {
        self.destroy_workers();
    }
}

pub struct SingleThreadedNodeUpdater {
    errors: Vec<NodeUpdateError>,
    eps: Option<u64>,
}

impl SingleThreadedNodeUpdater {
    pub fn new(eps: Option<u64>) -> Self {
        Self {
            errors: Vec::new(),
            eps: eps,
        }
    }
}

impl NodeUpdater for SingleThreadedNodeUpdater {
    #[tracing::instrument(skip_all, name = "single_threaded_update")]
    fn update(&mut self, node: (NodeId, Arc<Mutex<dyn RuntimeNode + Send>>), node_description: Option<NodeDescription>) {
        // TODO: Update instrumentation here
        if let Ok(mut n) = node.1.try_lock() {
            if let Err(err) = n.on_update() {
                self.errors.push(NodeUpdateError {
                    source: err,
                    node_id: Some(node.0),
                    node_desc: None,
                });
            }
        }
    }

    fn errors(&mut self) -> Vec<NodeUpdateError> {
        let drained_errors: Vec<NodeUpdateError> = self.errors.drain(..).collect();
        self.errors.clear();
        drained_errors
    }

    fn sleep_mode(&self) -> SleepMode {
        match self.eps {
            None => SleepMode::None,
            Some(eps) => SleepMode::FixedFrequency(eps),
        }
    }
}

impl Drop for SingleThreadedNodeUpdater {
    fn drop(&mut self) {}
}
