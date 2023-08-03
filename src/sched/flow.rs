use std::sync::{Arc, Mutex};
use anyhow::{Context, Result};

use crate::{sched::version::Version, node::Node};

pub struct Flow {
    name: String,
    version: Version,
    nodes: Vec<Arc<Mutex<dyn Node>>>,
}

impl Flow {
    pub fn new(name: &str, v: Version) -> Self {
        Self {
            name: name.to_string(),
            version: v,
            nodes: Vec::new(),
        }
    }

    pub fn add_node<T>(&mut self, node: T)
    where
        T: Node,
    {
        self.nodes.push(Arc::new(Mutex::new(node)));
    }

    pub fn get_node(&self, idx: usize) -> Option<Arc<Mutex<dyn Node>>> {
        self.nodes.get(idx).map(|node| node.clone())
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn init_all(&self) -> Result<()> {
        for n in &self.nodes {
            let name :String = n.lock().unwrap().name().to_string();
            n
                .lock()
                .unwrap()
                .on_init()
                .context(format!("Unable to init node '{}'.", name))?;
        }
        Ok(())
    }

    pub fn shutdown_all(&self) -> Result<()> {
        for n in &self.nodes {
            let name :String = n.lock().unwrap().name().to_string();
            n
                .lock()
                .unwrap()
                .on_shutdown()
                .context(format!("Unable to shutdown node '{}'.", name))?;
        }
        Ok(())

    }

    pub fn ready_all(&self) -> Result<()> {
        for n in &self.nodes {
            let name :String = n.lock().unwrap().name().to_string();
            n
                .lock()
                .unwrap()
                .on_ready()
                .context(format!("Unable to make node '{}' ready.", name))?;
        }
        Ok(())
    }
}
