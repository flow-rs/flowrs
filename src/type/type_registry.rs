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

    /// Register a type with a **custom factory function**
    pub fn register<T, F>(&mut self, factory: F)
    where
        T: 'static + Send + Sync + FromStr + Debug,
        NetworkCommunicator: Communicator<T>,
        F: Fn() -> T + Send + Sync + 'static,
    {
        if !self.creators.contains_key(&TypeId::of::<T>()) {
            self.creators
                .insert(TypeId::of::<T>(), Box::new(move || Box::new(factory())));
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

/// Register a type globally **with a custom factory function**
pub async fn register_global<T, F>(factory: F)
where
    T: 'static + Send + Sync + FromStr + Debug,
    NetworkCommunicator: Communicator<T>,
    F: Fn() -> T + Send + Sync + 'static, // Custom factory function
{
    unsafe {
        if let Some(registry) = &TYPE_REGISTRY {
            let mut reg = registry.lock().await;
            reg.register::<T, F>(factory);
        }
    }
}
