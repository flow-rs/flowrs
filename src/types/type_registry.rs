use crate::comm::communication::Communicator;
use crate::comm::communication::NodeCommunicator;
use crate::comm::data::DataWrapper;
use crate::comm::messages::Message;
use crate::comm::network_communicator::NetworkCommunicator;
use crate::nodes::node_io::SetupIO;
use async_trait::async_trait;
use futures::FutureExt;
use futures::TryFutureExt;
use futures_core::future::BoxFuture;
use lazy_static::lazy_static;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Mutex;

use crate::flow::flow_types::{NodeIOIndex, NodeId};

type ConnectionFn =
    fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn SetupIO, &mut dyn SetupIO);

#[async_trait]
pub trait CommunicatorBox: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    async fn send_boxed(&mut self, message: Box<dyn Any + Send>) -> Result<(), String>;

    async fn connect_send(&mut self, addr: &str, port: u16) -> Result<(), String>;
    async fn connect_receive(&mut self, port: u16) -> Result<(), String>;
    fn into_node_communicator(self: Box<Self>) -> Result<Box<dyn Any + Send>, String>;
}

#[async_trait]
impl<T: 'static + Send + Sync + Debug + FromStr> CommunicatorBox for NetworkCommunicator<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn send_boxed(&mut self, message: Box<dyn Any + Send>) -> Result<(), String> {
        match message.downcast::<T>() {
            Ok(msg) => {
                let wrapper = Message::Data(DataWrapper { data: *msg });
                self.send(wrapper).await.map_err(|e| e.to_string())
            }
            Err(_) => Err("Failed to downcast message".into()),
        }
    }

    async fn connect_send(&mut self, addr: &str, port: u16) -> Result<(), String> {
        Communicator::<T>::connect_send(self, Some(addr.to_string()), Some(port))
            .await
            .map_err(|e| e.to_string())
    }

    async fn connect_receive(&mut self, port: u16) -> Result<(), String> {
        Communicator::<T>::connect_recv(self, Some(port.to_string()), None)
            .await
            .map_err(|e| e.to_string())
    }

    fn into_node_communicator(mut self: Box<Self>) -> Result<Box<dyn Any + Send>, String> {
        if let Some(concrete) = self.as_any_mut().downcast_mut::<NetworkCommunicator<T>>() {
            // Move out the communicator (consume it) by replacing with a dummy (requires Option wrapping)
            let moved = std::mem::replace(concrete, NetworkCommunicator::dummy());
            Ok(Box::new(NodeCommunicator::NetworkComm(moved)))
        } else {
            Err("Failed to downcast to NetworkCommunicator<T>".to_string())
        }
    }
}

type AnyCommunicator = Box<dyn CommunicatorBox + Send + Sync>;
pub type CommunicatorFactory = fn() -> BoxFuture<'static, Box<dyn CommunicatorBox>>;

pub struct TypeRegistry {
    connections: HashMap<TypeId, ConnectionFn>,
    communicator_factories: HashMap<TypeId, CommunicatorFactory>,
    name_to_id: HashMap<String, TypeId>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            communicator_factories: HashMap::new(),
            name_to_id: HashMap::new(),
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

    pub fn register_communicator<T>(&mut self)
    where
        T: 'static + Send + Sync + Debug + FromStr,
        NetworkCommunicator<T>: Communicator<T> + CommunicatorBox,
    {
        let type_id = TypeId::of::<T>();
        self.communicator_factories.insert(type_id, || {
            async {
                let comm = NetworkCommunicator::<T>::new()
                    .await
                    .expect("Failed to create communicator");
                Box::new(comm) as Box<dyn CommunicatorBox>
            }
            .boxed()
        });
    }

    pub async fn create_communicator_by_name(
        &self,
        type_name: &str,
    ) -> Result<AnyCommunicator, String> {
        let type_id = self
            .name_to_id
            .get(type_name)
            .ok_or_else(|| format!("[TypeRegistry] Unknown type name: {}", type_name))?;

        let factory = self
            .communicator_factories
            .get(type_id)
            .ok_or_else(|| format!("No communicator factory for type: {}", type_name))?;

        Ok(factory().await)
    }
}

lazy_static! {
    pub static ref TYPE_REGISTRY: Mutex<TypeRegistry> = Mutex::new(TypeRegistry::new());
}
