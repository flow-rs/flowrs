use lazy_static::lazy_static;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::flow::flow_types::{NodeIOIndex, NodeId};

type ConnectionFn =
    fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn SetupIO, &mut dyn SetupIO);

pub struct TypeRegistry {
    connections: HashMap<TypeId, ConnectionFn>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }

    /// Register a type with its connection function
    pub fn register<T: 'static>(&mut self, func: ConnectionFn) {
        let type_id = TypeId::of::<T>();
        self.connections.insert(type_id, func);
        println!(
            "[DEBUG] Registered connection function for type {:?}",
            type_id
        );
    }

    /// Get the connection function based on type ID
    pub fn get(&self, type_id: TypeId) -> Option<&ConnectionFn> {
        self.connections.get(&type_id)
    }
}

lazy_static! {
    pub static ref TYPE_REGISTRY: Mutex<TypeRegistry> = Mutex::new(TypeRegistry::new());
}
