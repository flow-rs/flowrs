use std::{sync::{Arc, Mutex}, collections::HashMap};
use anyhow::{Context, Result};


use crate::{sched::version::Version, node::UpdateController, connection::RuntimeNode};

#[derive(Clone)]
pub struct Flow {
    name: String,
    version: Version,
    pub nodes: HashMap<String, Arc<Mutex<dyn RuntimeNode + Send>>>,
   
}

impl Flow {
    pub fn new(name: &str, v: Version, nodes: HashMap<String, Arc<Mutex<dyn RuntimeNode + Send>>>) -> Self {
        Self {
            name: name.to_string(),
            version: v,
            nodes,
        }
    }

    pub fn add_node<T>(&mut self, node: T, id: String)
    where
        T: RuntimeNode + 'static,
    {
        self.nodes.insert(id, Arc::new(Mutex::new(node)));
    }

    pub fn get_node(&self, idx: usize) -> Option<Arc<Mutex<dyn RuntimeNode + Send>>> {
        let mut keys = self.nodes.keys().into_iter().collect::<Vec<&String>>();
        keys.sort();
        let key = *keys.get(idx).unwrap();
        self.nodes.get(key).map(|node| node.clone())
    }

    pub fn get_key(&self, node: Arc<Mutex<dyn RuntimeNode + Send>>) -> Option<&String> {
        self.nodes.iter()
        .find_map(|(key, &ref val)| if Arc::ptr_eq(&val, &node) { Some(key) } else { None })
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn init_all(&self) -> Result<()> {
        for n in self.nodes.values() {
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
        for n in self.nodes.values() {
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
        for n in self.nodes.values() {
            let name :String = n.lock().unwrap().name().to_string();
            n
                .lock()
                .unwrap()
                .on_ready()
                .context(format!("Unable to make node '{}' ready.", name))?;
        }
        Ok(())
    }

    pub fn get_update_controllers(&self) -> Vec<Arc<Mutex<dyn UpdateController>>> {
        let mut update_controllers = Vec::new(); 
        for node in self.nodes.values() {
            if let Some(us) = node.lock().unwrap().update_controller() {
                update_controllers.push(us.clone());
            }
        }
        update_controllers
    }

}
