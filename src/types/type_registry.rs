use crate::comm::communication::Communicator;
use crate::comm::communication::NodeCommunicator;
use crate::comm::data::DataWrapper;
use crate::comm::messages::Message;
#[cfg(not(target_arch = "wasm32"))]
use crate::comm::network_communicator::NetworkCommunicator;
use crate::node::ReceiveError;
use crate::nodes::node_io::SettableCommunicator;
use crate::nodes::node_io::SetupIO;
use crate::nodes::node_io::TypedInput;
use crate::nodes::node_io::TypedOutput;
use anyhow::anyhow;
use async_trait::async_trait;
use futures::FutureExt;
use futures_core::future::BoxFuture;
use lazy_static::lazy_static;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use tokio::sync::Mutex;

use crate::flow::flow_types::{NodeIOIndex, NodeId};

type ConnectionFn =
    fn(NodeId, NodeId, NodeIOIndex, NodeIOIndex, &mut dyn SetupIO, &mut dyn SetupIO);
type AnyCommunicator = Box<dyn CommunicatorBox + Send + Sync>;
pub type CommunicatorFactory = fn() -> BoxFuture<'static, Box<dyn CommunicatorBox>>;
type InputSetterFn = fn(
    node_io: &mut dyn SetupIO,
    index: NodeIOIndex,
    communicator: Box<dyn Any + Send>,
) -> Result<(), String>;
type OutputSetterWithConnectFn =
    for<'a> fn(
        &'a mut dyn SetupIO,
        NodeIOIndex,
        String,
        u16,
    ) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>;
pub type PollResult = Result<(), ReceiveError<String>>;
pub type PollFn<T> = Box<
    dyn for<'a> FnMut(
            &'a mut dyn SetupIO,
            NodeIOIndex,
        )
            -> Pin<Box<dyn Future<Output = Result<(), ReceiveError<T>>> + Send + 'a>>
        + Send,
>;

#[async_trait]
pub trait CommunicatorBox: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    async fn send_boxed(&mut self, message: Box<dyn Any + Send>) -> Result<(), String>;

    async fn connect_send(&mut self, addr: &str, port: u16) -> Result<(), String>;
    async fn connect_receive(&mut self, addr: &str, port: u16) -> Result<(), String>;
    fn into_node_communicator(self: Box<Self>) -> Result<Box<dyn Any + Send>, String>;
}

#[async_trait]
#[cfg(not(target_arch = "wasm32"))]
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

    async fn connect_receive(&mut self, addr: &str, port: u16) -> Result<(), String> {
        Communicator::<T>::connect_recv(self, Some(addr.to_string()), Some(port))
            .await
            .map_err(|e| e.to_string())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn into_node_communicator(self: Box<Self>) -> Result<Box<dyn Any + Send>, String> {
        // First cast self into a Box<dyn Any>
        let boxed_any = self as Box<dyn Any>;

        // Then downcast that into the concrete type
        match boxed_any.downcast::<NetworkCommunicator<T>>() {
            Ok(concrete) => Ok(Box::new(NodeCommunicator::NetworkComm(*concrete))),
            Err(_) => Err("Failed to downcast to NetworkCommunicator<T>".to_string()),
        }
    }
}

pub struct TypeRegistry {
    connections: HashMap<TypeId, ConnectionFn>,
    communicator_factories: HashMap<TypeId, CommunicatorFactory>,
    pub name_to_id: HashMap<String, TypeId>,
    input_setters: HashMap<TypeId, InputSetterFn>,
    output_setters_with_connect: HashMap<TypeId, OutputSetterWithConnectFn>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            communicator_factories: HashMap::new(),
            name_to_id: HashMap::new(),
            input_setters: HashMap::new(),
            output_setters_with_connect: HashMap::new(),
            //poll_fns: HashMap::new(),
        }
    }

    /// Register a type with its connection function
    pub fn register<T: 'static>(&mut self, func: ConnectionFn) {
        let type_id = TypeId::of::<T>();
        self.connections.insert(type_id, func);
        tracing::debug!(
            "[DEBUG] Registered connection function for type {:?}",
            type_id
        );
    }

    /// Get the connection function based on type ID
    pub fn get(&self, type_id: TypeId) -> Option<&ConnectionFn> {
        self.connections.get(&type_id)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn register_communicator<T>(&mut self, type_name: &str)
    where
        T: 'static + Send + Sync + Debug + FromStr + Clone,
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

        // Register Input Setters
        self.input_setters
            .insert(type_id, |node_io, idx, communicator| {
                if let Some(any_input) = node_io.get_input_communicator(idx) {
                    if let Some(input) = any_input.downcast_mut::<TypedInput<T>>() {
                        input.set_any_communicator(communicator);
                        Ok(())
                    } else {
                        Err("Failed to downcast to TypedInput".to_string())
                    }
                } else {
                    Err(format!("No input found at index {}", idx))
                }
            });

        let length_before = self.name_to_id.len();
        self.name_to_id.insert(type_name.to_string(), type_id);
        let length_after = self.name_to_id.len();
        tracing::debug!("[TYPE_REGISTRY] Inserting type name {} into name_to_id map. Length before: {}, Length After: {}", type_name.to_string(), length_before, length_after);

        // Register Ourput setters
        self.output_setters_with_connect
            .insert(type_id, |node_io, idx, ip, port| {
                Box::pin(async move {
                    if let Some(output_any) = node_io.get_output_communicator(idx) {
                        if let Some(output) = output_any.downcast_mut::<TypedOutput<T>>() {
                            let mut comm = NetworkCommunicator::<T>::new()
                                .await
                                .map_err(|e| format!("Failed to create communicator: {}", e))?;

                            Communicator::connect_send(&mut comm, Some(ip), Some(port))
                                .await
                                .map_err(|e| format!("Failed to connect: {}", e))?;

                            output.set_any_communicator(Box::new(NodeCommunicator::NetworkComm(
                                comm,
                            )));
                            Ok(())
                        } else {
                            Err("Could not downcast to TypedOutput".to_string())
                        }
                    } else {
                        Err("No output found at index".to_string())
                    }
                })
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

    pub async fn set_output_comm_with_connection(
        &self,
        type_name: &str,
        node_io: &mut dyn SetupIO,
        idx: NodeIOIndex,
        receiver_ip: String,
        port: u16,
    ) -> Result<(), String> {
        let type_id = self
            .name_to_id
            .get(type_name)
            .ok_or_else(|| format!("Unknown type name: {}", type_name))?;

        let setter = self
            .output_setters_with_connect
            .get(type_id)
            .ok_or_else(|| format!("No output setter for type: {}", type_name))?;

        setter(node_io, idx, receiver_ip, port).await
    }

    pub fn set_input_comm(
        &self,
        type_name: &str,
        node_io: &mut dyn SetupIO,
        input_idx: NodeIOIndex,
        communicator: Box<dyn Any + Send>,
    ) -> Result<(), String> {
        let type_id = self
            .name_to_id
            .get(type_name)
            .ok_or_else(|| format!("Unknown type name: {}", type_name))?;

        let setter = self
            .input_setters
            .get(type_id)
            .ok_or_else(|| format!("No input setter for type: {}", type_name))?;

        setter(node_io, input_idx, communicator)
    }
}

#[async_trait]
pub trait PollFnErased: Send {
    async fn poll(
        &mut self,
        io: &mut dyn SetupIO,
        index: NodeIOIndex,
    ) -> Result<(), ReceiveError<String>>;

    fn poll_indexed<'a>(
        &mut self,
        io: &'a mut dyn SetupIO,
        index: NodeIOIndex,
    ) -> Pin<Box<dyn Future<Output = Result<(), ReceiveError<String>>> + Send + 'a>>;
}

#[async_trait]
impl<T> PollFnErased for PollFn<T>
where
    T: 'static + Send + Clone + Debug + FromStr + Sync,
{
    async fn poll(
        &mut self,
        io: &mut dyn SetupIO,
        index: NodeIOIndex,
    ) -> Result<(), ReceiveError<String>> {
        tracing::debug!("[PollFn] Polling index {}...", index);
        let result = (self)(io, index).await;
        tracing::debug!("[PollFn] Poll result: {:?}", result);
        result.map_err(|e| ReceiveError::Other(anyhow!("{:?}", e)))
    }

    fn poll_indexed<'a>(
        &mut self,
        io: &'a mut dyn SetupIO,
        index: NodeIOIndex,
    ) -> Pin<Box<dyn Future<Output = Result<(), ReceiveError<String>>> + Send + 'a>> {
        Box::pin(async move {
            // Step 1: Retrieve the communicator (as dyn Any)
            let comm = io
                .get_input_communicator(index)
                .ok_or_else(|| anyhow!("Missing input communicator"))?;

            // Step 2: Get the type info before mutably borrowing comm
            let actual_type_id = (*comm).type_id();

            // Step 3: Attempt downcast to correct input type
            let typed_input = comm.downcast_mut::<TypedInput<T>>().ok_or_else(|| {
                tracing::error!(
                    "[DEBUG] Downcast failed! Expected: TypedInput<{}>, but got type ID: {:?}",
                    std::any::type_name::<T>(),
                    actual_type_id,
                );
                anyhow!(
                    "Downcast to TypedInput<{}> failed",
                    std::any::type_name::<T>()
                )
            })?;

            // Step 4: Poll and buffer
            typed_input
                .input
                .edge
                .poll_and_buffer()
                .await
                .map_err(|e| ReceiveError::Other(anyhow::anyhow!("{:?}", e)))?;

            Ok(())
        })
    }
}

/// Registers a poll function that flushes outputs for type `T`
pub fn register_flush_fn<T>()
where
    T: 'static + Send + Sync + Clone + Debug + FromStr,
{
    use crate::connection::Output;
    use crate::nodes::node_io::TypedOutput;

    let flush_fn: PollFn<T> = Box::new(|io, index| {
        Box::pin(async move {
            let output_any = io
                .get_output_communicator(index)
                .ok_or_else(|| anyhow!("Output communicator missing at index {}", index))?;

            let typed = output_any.downcast_mut::<TypedOutput<T>>().ok_or_else(|| {
                anyhow!(
                    "Failed to downcast to TypedOutput<{}>",
                    std::any::type_name::<T>()
                )
            })?;

            typed
                .output
                .flush()
                .await
                .map_err(|e| ReceiveError::Other(anyhow!(e.to_string())))?;

            Ok(())
        })
    });

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut registry = POLL_REGISTRY.blocking_lock();
        registry.register_poll_fn::<T>(flush_fn);
    }

    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let mut registry = POLL_REGISTRY.lock().await;
            registry.register_poll_fn::<T>(flush_fn);
        });
    }
}

pub struct PollRegistry {
    poll_fns: HashMap<TypeId, Box<dyn PollFnErased>>,
}

impl PollRegistry {
    pub fn new() -> Self {
        Self {
            poll_fns: HashMap::new(),
        }
    }

    pub fn register_poll_fn<T>(&mut self, poll_fn: PollFn<T>)
    where
        T: 'static + Send + Sync + Clone + Debug + FromStr,
    {
        self.poll_fns.insert(
            TypeId::of::<T>(),
            Box::new(poll_fn) as Box<dyn PollFnErased>,
        );
    }

    pub fn get_mut(&mut self, type_id: &TypeId) -> Option<&mut Box<dyn PollFnErased>> {
        self.poll_fns.get_mut(type_id)
    }

    pub fn get_poll_fn_erased(&mut self, type_id: &TypeId) -> Option<&mut Box<dyn PollFnErased>> {
        self.poll_fns.get_mut(type_id)
    }
}

lazy_static! {
    pub static ref POLL_REGISTRY: Mutex<PollRegistry> = Mutex::new(PollRegistry::new());
}

lazy_static! {
    pub static ref TYPE_REGISTRY: Mutex<TypeRegistry> = Mutex::new(TypeRegistry::new());
}
