use crate::comm::communication::Communicator;
use crate::comm::network_communicator::NetworkCommunicator;
use crate::comm::thread_communicator::ThreadCommunicator;
use async_trait::async_trait;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;

/// Type Registry for mapping TypeId to factory functions
pub struct TypeRegistry {
    /// For each TypeId, store a pair of closures:
    ///  (local_creator, remote_creator)
    creators: HashMap<
        TypeId,
        (
            Box<dyn Fn() -> Box<dyn CommunicatorBox> + Send + Sync>,
            Box<dyn Fn() -> Box<dyn CommunicatorBox> + Send + Sync>,
        ),
    >,
}

impl TypeRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            creators: HashMap::new(),
        }
    }

    /// Register both local & remote creators for a given type T
    pub fn register<T>(&mut self)
    where
        T: 'static + Send + Sync + Debug + FromStr + Clone,
        ThreadCommunicator<T>: Communicator<T>,
        NetworkCommunicator: Communicator<T>,
    {
        let type_id = TypeId::of::<T>();
        if !self.creators.contains_key(&type_id) {
            let local_closure = Box::new(|| {
                Box::new(ThreadCommunicator::<T>::new().unwrap()) as Box<dyn CommunicatorBox>
            })
                as Box<dyn Fn() -> Box<dyn CommunicatorBox> + Send + Sync>;

            let remote_closure = Box::new(|| {
                let rt = Runtime::new().expect("Failed to create tokio runtime");
                let net_comm = rt
                    .block_on(NetworkCommunicator::new())
                    .expect("Failed to create NetworkCommunicator asynchronously");

                Box::new(net_comm) as Box<dyn CommunicatorBox>
            })
                as Box<dyn Fn() -> Box<dyn CommunicatorBox> + Send + Sync>;
            self.creators
                .insert(type_id, (local_closure, remote_closure));
        }
    }

    /// Create *one* communicator (either local or remote) by type_id
    fn create_communicator(
        &self,
        type_id: &TypeId,
        local: bool,
    ) -> Option<Box<dyn CommunicatorBox>> {
        let (local_creator, remote_creator) = self.creators.get(type_id)?;
        let creator = if local { local_creator } else { remote_creator };
        Some((creator)())
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

pub async fn get_thread_communicator(type_id: TypeId, local: bool) -> Option<Box<dyn Any>> {
    let reg = unsafe { TYPE_REGISTRY.as_ref()? }; // Ensure TYPE_REGISTRY is Some()
    let reg_guard = reg.lock().await; // Lock the Mutex safely
    let comm_box = reg_guard.create_communicator(&type_id, local)?; // Pass local flag
    Some(comm_box.as_any_box())
}

pub async fn get_network_communicator<T: 'static + Send + Sync + Debug + FromStr>(
) -> Option<Box<NetworkCommunicator>>
where
    NetworkCommunicator: Communicator<T>,
{
    unsafe {
        let registry_ref = TYPE_REGISTRY.as_ref()?;
        let reg = registry_ref.lock().await;
        // we pass `false` for remote
        let comm_box = reg.create_communicator(&TypeId::of::<T>(), /*local=*/ false)?;
        // downcast from Box<dyn CommunicatorBox> -> Box<dyn Any> -> Box<NetworkCommunicator>
        let any_boxed = comm_box.as_any_box();
        any_boxed.downcast::<NetworkCommunicator>().ok()
    }
}

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
