use std::{thread, sync::{ Arc, Mutex}};

use crate::{
    node::Node,
    flow::version::Version
};

pub struct Flow {
    name: String, 
    version: Version,
    nodes: Vec<Arc<Mutex<dyn Node>>>
}

impl Flow {

    pub fn new(name: &str, v:Version) -> Self {
        Self { name: name.to_string(), version: v, nodes: Vec::new() }
    }

    pub fn add_node<T>(&mut self, node: T)
    where T: Node {
        self.nodes.push(Arc::new(Mutex::new(node)));
    }

    pub fn get_node(&self, idx: usize) -> Option<Arc<Mutex<dyn Node>>> {
        self.nodes.get(idx).map(|node| node.clone())
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn init_all(&self) {
        for n in &self.nodes {
            n.lock().unwrap().on_init();
        }
    }

    pub fn shutdown_all(&self) {
        for n in &self.nodes {
            n.lock().unwrap().on_shutdown();
        }
    }

    pub fn ready_all(&self) {
        for n in &self.nodes {
            n.lock().unwrap().on_ready();
        }
    }
}
