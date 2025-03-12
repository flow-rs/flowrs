use crate::comm::communication::Communicator;
use crate::comm::network_communicator::NetworkCommunicator;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Type Registry for mapping TypeId to factory functions
pub struct TypeRegistry {
    creators: HashMap<TypeId, Box<dyn Fn() -> Box<dyn Any> + Send + Sync>>,
}

impl TypeRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
        }
    }

    /// Register a type with a factory function
    pub fn register<T: 'static + Send + Sync + FromStr + Debug + Clone>(&mut self)
    where
        NetworkCommunicator: Communicator<T>,
    {
        if !self.creators.contains_key(&TypeId::of::<T>()) {
            self.creators.insert(
                TypeId::of::<T>(),
                Box::new(|| Box::new(NetworkCommunicator::new())),
            );
        }
    }

    /// Retrieve a new instance based on `TypeId`
    pub fn create_instance(&self, type_id: &TypeId) -> Option<Box<dyn Any>> {
        self.creators.get(type_id).map(|f| f())
    }
}

/// Global registry
pub static mut TYPE_REGISTRY: Option<Arc<Mutex<TypeRegistry>>> = None;

/// Initialize global registry (called at startup)
pub fn initialize_registry() {
    unsafe {
        TYPE_REGISTRY = Some(Arc::new(Mutex::new(TypeRegistry::new())));
    }
}

/// Register a type globally
pub async fn register_global<T: 'static + Send + Sync + FromStr + Debug + Clone>()
where
    NetworkCommunicator: Communicator<T>,
{
    unsafe {
        if let Some(registry) = &TYPE_REGISTRY {
            let mut reg = registry.lock().await;
            reg.register::<T>();
        }
    }
}
