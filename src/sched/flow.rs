use std::{sync::{Arc, Mutex}, collections::HashMap, hash::Hash};
use anyhow::{Context, Result};


use crate::{sched::version::Version, node::{UpdateController}, nodes::node_description::NodeDescription,  connection::RuntimeNode};

pub struct Flow<T> where T : std::hash::Hash {
    name: String,
    version: Version,
    
    nodes: Vec<(NodeDescription, Arc<Mutex<dyn RuntimeNode + Send>>)>,
    id_to_node_idx: HashMap<T, usize>,  
}

impl<T> Flow<T> 
    where T: Hash + Eq + PartialEq{
    
    pub fn new_empty(name: &str, v: Version) -> Self {
        Self {
            name: name.to_string(),
            version: v,
            nodes: Vec::new(),
            id_to_node_idx : HashMap::new()
        }
    }

    pub fn new(name: &str, v: Version, nodes: HashMap<T, (NodeDescription, Arc<Mutex<dyn RuntimeNode + Send>>)>) -> Self {
        let mut obj = Self {
            name: name.to_string(),
            version: v,
            nodes: Vec::new(),
            id_to_node_idx : HashMap::new()
        };

        for (id, (node_description, runtime_node)) in nodes {
            let node_idx = obj.nodes.len();
            obj.nodes.push((node_description, runtime_node.clone()));
            obj.id_to_node_idx.insert(id, node_idx);
        }

        obj
    }

    pub fn add_node<U>(&mut self, node: U, id: T)
    where
        U: RuntimeNode + 'static,
    {
        self.add_node_with_desc(node, id, NodeDescription::default());
    }

    pub fn add_node_with_desc<U>(&mut self, node: U, id: T, desc: NodeDescription)
    where
        U: RuntimeNode + 'static,
    {
        self.nodes.push((desc, Arc::new(Mutex::new(node))));
        self.id_to_node_idx.insert(id, self.nodes.len()-1);
    }


    pub fn node_by_index(&self, index: usize) -> Option<&(NodeDescription, Arc<Mutex<dyn RuntimeNode + Send>>)>{
        self.nodes.get(index)
    }

    pub fn node_by_id(&self, id: &T) -> Option<&(NodeDescription, Arc<Mutex<dyn RuntimeNode + Send>>)>{
        if let Some(idx) = self.id_to_node_idx.get(id){
            return self.node_by_index(*idx);
        }
        None
    }

    pub fn num_nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn init_all(&self) -> Result<()> {
        for n in &self.nodes {
            n.1
                .lock()
                .unwrap()
                .on_init()
                .context(format!("Unable to init node '{}'.", n.0.name))?;
        }
        Ok(())
    }

    pub fn shutdown_all(&self) -> Result<()> {
        for n in &self.nodes {
            n.1
                .lock()
                .unwrap()
                .on_shutdown()
                .context(format!("Unable to shutdown node '{}'.",  n.0.name))?;
        }
        Ok(())
    }

    pub fn ready_all(&self) -> Result<()> {
        for n in &self.nodes {
            let name: String = "".into(); 

            n.1
                .lock()
                .unwrap()
                .on_ready()
                .context(format!("Unable to make node '{}' ready.", n.0.name))?;
        }
        Ok(())
    }

    pub fn get_update_controllers(&self) -> Vec<Arc<Mutex<dyn UpdateController>>> {
        let mut update_controllers = Vec::new(); 
        for n in &self.nodes {
            if let Some(us) = n.1.lock().unwrap().update_controller() {
                update_controllers.push(us.clone());
            }
        }
        update_controllers
    }

}
