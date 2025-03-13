use crate::comm::communication::Communicator;
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Type Registry for mapping TypeId to factory functions
pub struct TypeRegistry {
    creators: HashMap<TypeId, Box<dyn Fn() -> Box<dyn CommunicatorBox> + Send + Sync>>,
}

impl TypeRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
        }
    }

    /// Register a type with a factory function
    pub fn register<T: 'static + Send + Sync + Debug + FromStr + Clone>(&mut self)
    where
        NetworkCommunicator: Communicator<T>,
        ThreadCommunicator<T>: Communicator<T>,
    {
        let type_id = TypeId::of::<T>();
        if !self.creators.contains_key(&type_id) {
            self.creators.insert(
                type_id,
                Box::new(|| {
                    Box::new(ThreadCommunicator::<T>::new().unwrap()) as Box<dyn CommunicatorBox>
                }),
            );
        }
    }

    /// Retrieve a new instance based on `TypeId`
    pub fn create_instance(&self, type_id: &TypeId) -> Option<Box<dyn Any>> {
        self.creators.get(type_id).map(|creator| {
            let boxed = creator(); // Get Box<dyn CommunicatorBox>
            boxed.as_any_box() // Convert it to Box<dyn Any>
        })
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
pub async fn register_global<T: 'static + Send + Sync + Debug + FromStr + Clone>()
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

/// Register only base types `T` instead of `Input<T>` and `Output<T>`
pub async fn register_base_type<T: 'static + Send + Sync + Debug + FromStr + Clone>()
where
    NetworkCommunicator: Communicator<T>,
{
    register_global::<T>().await;
}

pub async fn get_thread_communicator<T: 'static + Send + Sync + Debug + FromStr>(
) -> Option<Box<ThreadCommunicator<T>>>
where
    ThreadCommunicator<T>: Communicator<T>,
{
    unsafe {
        if let Some(registry) = &TYPE_REGISTRY {
            let reg = registry.lock().await;
            if let Some(boxed) = reg.creators.get(&TypeId::of::<T>()).map(|f| f()) {
                return boxed.as_any_box().downcast::<ThreadCommunicator<T>>().ok();
            }
        }
    }
    None
}

pub async fn get_network_communicator<T: 'static + Send + Sync + Debug + FromStr>(
) -> Option<Box<NetworkCommunicator>>
where
    NetworkCommunicator: Communicator<T>,
{
    unsafe {
        if let Some(registry) = &TYPE_REGISTRY {
            let reg = registry.lock().await;
            if let Some(boxed) = reg.creators.get(&TypeId::of::<T>()).map(|f| f()) {
                return boxed.as_any_box().downcast::<NetworkCommunicator>().ok();
            }
        }
    }
    None
}

// pub async fn get_typed_communicator<T: 'static + Send + Sync + Debug + FromStr>(
// ) -> Option<Box<dyn Communicator<T>>>
// where
//     NetworkCommunicator: Communicator<T>,
// {
//     get_communicator::<T>().await?.downcast().ok()
// }

pub trait CommunicatorBox: Send + Sync + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_mut_any(&mut self) -> &mut dyn Any;
    fn as_any_box(self: Box<Self>) -> Box<dyn Any>;
}

impl<T> CommunicatorBox for ThreadCommunicator<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self //Converts `Box<dyn CommunicatorBox>` to `Box<dyn Any>`
    }
}

impl CommunicatorBox for NetworkCommunicator {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn Any {
        self
    }
    fn as_any_box(self: Box<Self>) -> Box<dyn Any> {
        self //Converts `Box<dyn CommunicatorBox>` to `Box<dyn Any>`
    }
}
