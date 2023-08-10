use std::{
    sync::{ Arc, Mutex}, thread::{self, JoinHandle},
};
use crate::{
    node::{UpdateError},
    connection::RuntimeNode,
};
use crossbeam_channel::{unbounded, Sender, Receiver};

pub enum SleepMode {
    Reactive, 
    FixedFrequency(u64),
    None
}

enum WorkerCommand {
    Update(Arc<Mutex<dyn RuntimeNode + Send>>),
    Cancel
}

pub trait NodeUpdater {
    fn update(&mut self, node: Arc<Mutex<dyn RuntimeNode + Send>>);
    fn errors(&mut self) -> Vec<UpdateError>;

    fn sleep_mode(&self) -> SleepMode;
}

pub struct MultiThreadedNodeUpdater {
    num_workers: usize,
    workers: Vec<JoinHandle<Result<(), UpdateError>>>,

    command_channel: (Sender<WorkerCommand>, Receiver<WorkerCommand>),
    error_channel: (Sender<UpdateError>, Receiver<UpdateError>)
}

impl MultiThreadedNodeUpdater {

    pub fn new(num_workers: usize) -> Self {
        let mut obj = Self {
            num_workers: num_workers,
            workers: Vec::new(),
            
            command_channel: unbounded(),
            error_channel:  unbounded()
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

    fn create_workers(&mut self){

        for _ in 0..self.num_workers {

            let update_receiver_clone = self.command_channel.1.clone();
            let error_sender_clone = self.error_channel.0.clone();

            let thread_handle: thread::JoinHandle<Result<(), UpdateError>> = thread::spawn( move || -> Result<(), UpdateError>  {
                
                loop {

                    let update_receiver_res = update_receiver_clone.recv();
                    match update_receiver_res {
                        
                        Result::Err(err) => {
                            //println!("{:?} THREAD UPDATE ERROR {:?}", std::thread::current().id(), err);
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
                                    
                                    if let Ok(mut n) = node.try_lock() {
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

impl NodeUpdater for MultiThreadedNodeUpdater {
    
    fn update(&mut self, node: Arc<Mutex<dyn RuntimeNode + Send>>) {
        self.command_channel.0.send(WorkerCommand::Update(node.clone())).expect("Unable to write to command channel.");
    }

    fn errors(&mut self) -> Vec<UpdateError> {
       let errors: Vec<UpdateError> = self.error_channel.1.try_iter().collect();
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
    errors: Vec<UpdateError>,
    eps: Option<u64>
}

impl SingleThreadedNodeUpdater {
    pub fn new(eps: Option<u64>) -> Self {
        Self {
            errors: Vec::new(),
            eps: eps
        }
    }
}

impl NodeUpdater for SingleThreadedNodeUpdater {
    
    fn update(&mut self, node: Arc<Mutex<dyn RuntimeNode + Send>>) {
        if let Ok(mut n) = node.try_lock() {
            if let Err(err) = n.on_update() {
                self.errors.push(err);
            }
        }
    }

    fn errors(&mut self) -> Vec<UpdateError> {
        let drained_errors: Vec<UpdateError> = self.errors.drain(..).collect();
        self.errors.clear();
        drained_errors
    }

    fn sleep_mode(&self) -> SleepMode {
        match self.eps {
            None => SleepMode::None,
            Some(eps) => SleepMode::FixedFrequency(eps)
        }
    }
}

impl Drop for SingleThreadedNodeUpdater {
    fn drop(&mut self) {
    }
}