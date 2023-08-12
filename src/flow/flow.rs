use std::{sync::{Arc, Mutex}, collections::HashMap};
use anyhow::{Context, Result};


use crate::{flow::version::Version, node::UpdateController, nodes::node_description::NodeDescription,  connection::RuntimeNode};

pub type NodeId = u128;

pub struct Flow { 
    name: String,
    version: Version,
    
    nodes: Vec<(NodeId, Arc<Mutex<dyn RuntimeNode + Send>>)>,
    id_to_node_idx: HashMap<NodeId, usize>,  
    id_to_desc: HashMap<NodeId, NodeDescription>,
    id_counter: NodeId
}

impl Flow {
    
    pub fn new_empty(name: &str, v: Version) -> Self {
        Self {
            name: name.to_string(),
            version: v,
            nodes: Vec::new(),
            id_to_node_idx : HashMap::new(),
            id_to_desc: HashMap::new(),
            id_counter: 0
        }
    }

    pub fn new(name: &str, v: Version, nodes: HashMap<NodeId, Arc<Mutex<dyn RuntimeNode + Send>>>) -> Self {
        let mut obj = Self {
            name: name.to_string(),
            version: v,
            nodes: Vec::new(),
            id_to_node_idx : HashMap::new(),
            id_to_desc: HashMap::new(),
            id_counter: 0
        };

        for (id, runtime_node) in nodes {
            let node_idx = obj.nodes.len();
            obj.nodes.push((id,runtime_node));
            obj.id_to_node_idx.insert(id, node_idx);
        }

        obj
    }

    fn generate_id(&mut self) -> NodeId  {
        self.id_counter += 1;
        self.id_counter
    }

    pub fn node_description_by_id(&self, id: NodeId) -> Option<&NodeDescription> {
        self.id_to_desc.get(&id)
    }

    pub fn add_node<T>(&mut self, node: T) -> NodeId
    where
        T: RuntimeNode + 'static {
        let id = self.generate_id();
        self.add_node_with_id_and_desc(node, id, NodeDescription::default())
    }

    pub fn add_node_with_id<T>(&mut self, node: T, id: NodeId)-> NodeId
    where
        T: RuntimeNode + 'static {
        self.add_node_with_id_and_desc(node, id, NodeDescription::default())
    }
    
    pub fn add_node_with_id_and_desc<T>(&mut self, node: T, id: NodeId, desc: NodeDescription) -> NodeId
    where
        T: RuntimeNode + 'static {
        if !self.id_to_node_idx.contains_key(&id) {
                
            self.nodes.push((id, Arc::new(Mutex::new(node))));
            self.id_to_node_idx.insert(id, self.nodes.len()-1);
            self.id_to_desc.insert(id, desc);
        }

        id
    }

    pub fn node_by_index(&self, index: usize) -> Option<&(NodeId, Arc<Mutex<dyn RuntimeNode + Send>>)>{
        self.nodes.get(index)
    }

    pub fn node_by_id(&self, id: NodeId) -> Option<&(NodeId, Arc<Mutex<dyn RuntimeNode + Send>>)>{
        if let Some(idx) = self.id_to_node_idx.get(&id){
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
                .context(format!("Unable to init node with ID {}.", n.0))?;
        }
        Ok(())
    }

    pub fn shutdown_all(&self) -> Result<()> {
        for n in &self.nodes {
            n.1
                .lock()
                .unwrap()
                .on_shutdown()
                .context(format!("Unable to shutdown node with ID {}.", n.0))?;
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
                .context(format!("Unable to make node with ID {}.", n.0))?;
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
