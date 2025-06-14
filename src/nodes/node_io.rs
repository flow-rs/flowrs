use crate::comm::thread_communicator::{Splittable, ThreadCommunicator};
use crate::flow::flow_types::NodeIOIndex;
use async_trait::async_trait;
use std::any::Any;
use std::{fmt::Debug, str::FromStr};
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;

use super::connection::Input;
use super::connection::Output;
use super::connection::{Edge, EdgeTrait};
use crate::comm::communication::{Communicator, NodeCommunicator};

/// The main I/O wrapper for all node implementations
#[derive(Debug)]
pub struct NodeIO<I, O>
where
    I: SetupInputs,
    O: SetupOutputs,
{
    pub inputs: I,
    pub outputs: O,
}

impl<I, O> NodeIO<I, O>
where
    I: SetupInputs + SetupInputsSync + Send + Sync,
    O: SetupOutputs + SetupOutputsSync + Send + Sync,
{
    pub fn new(inputs: I, outputs: O) -> Self {
        Self { inputs, outputs }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_input_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.inputs.setup_input(idx, local));
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_output_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.outputs.setup_output(idx, local));
    }

    #[cfg(target_arch = "wasm32")]
    pub fn setup_input_sync(&mut self, _idx: u128, _local: bool) {
        panic!("setup_input_sync is not supported on wasm");
    }

    #[cfg(target_arch = "wasm32")]
    pub fn setup_output_sync(&mut self, _idx: u128, _local: bool) {
        panic!("setup_output_sync is not supported on wasm");
    }
}

//================================SETUP TRAITS========================================================

/// Traits for setting up inputs and outputs asynchronously
#[async_trait]
pub trait SetupInputs {
    async fn setup_input(&mut self, idx: NodeIOIndex, local: bool);
}

#[async_trait]
pub trait SetupOutputs {
    async fn setup_output(&mut self, idx: NodeIOIndex, local: bool);
}

/// **Synchronous wrapper traits**
pub trait SetupInputsSync {
    fn setup_input_sync(&mut self, idx: NodeIOIndex, local: bool);
    fn get_input_count(&self) -> NodeIOIndex;
}

pub trait SetupOutputsSync {
    fn setup_output_sync(&mut self, idx: NodeIOIndex, local: bool);
    fn get_output_count(&self) -> NodeIOIndex;
}

#[async_trait]
impl<I> SetupInputs for Input<I>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
{
    async fn setup_input(&mut self, idx: NodeIOIndex, local: bool) {
        *self = if local {
            Input::new_local()
        } else {
            Input::new_network().await
        };
    }
}

impl<I, O> NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs + 'static,
    O: SetupOutputsSync + SetupOutputs + 'static,
{
    pub fn get_io_mut(&mut self) -> &mut Self {
        self
    }
}

#[async_trait]
impl<O> SetupOutputs for Output<O>
where
    O: 'static + Send + Sync + Debug + FromStr + Clone,
{
    async fn setup_output(&mut self, idx: u128, local: bool) {
        *self = if local {
            Output::new_local()
        } else {
            Output::new_network().await
        };
    }
}

impl<D> SetupInputsSync for Input<D>
where
    D: 'static + Send + Sync + Debug + FromStr + Clone,
{
    #[cfg(not(target_arch = "wasm32"))]
    fn setup_input_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.setup_input(idx, local));
    }

    #[cfg(target_arch = "wasm32")]
    fn setup_input_sync(&mut self, _idx: u128, _local: bool) {
        panic!("setup_input_sync is not supported on wasm");
    }

    fn get_input_count(&self) -> NodeIOIndex {
        1 // Single input
    }
}

#[async_trait]
impl SetupOutputs for () {
    async fn setup_output(&mut self, _idx: NodeIOIndex, _local: bool) {
        // nothing to set up
    }
}

/// **Macro to generate `SetupInputs` implementations**
#[macro_export]
macro_rules! impl_setup_inputs {
    (() $(,)?) => {
        #[async_trait]
        impl SetupInputs for () {
            async fn setup_input(&mut self, _idx: NodeIOIndex, _local: bool) {}
        }

        impl SetupInputsSync for () {
            fn setup_input_sync(&mut self, _idx: NodeIOIndex, _local: bool) {}
            fn get_input_count(&self) -> NodeIOIndex {
                0 // no input
            }

        }
    };

    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait]
        impl<$($D),+> SetupInputs for ($($crate::nodes::node_io::TypedInput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            async fn setup_input(&mut self, idx: NodeIOIndex, local: bool) {
                match idx {
                    $(
                        $idx => self.$idx.input.setup_input(idx, local).await,
                    )+
                    _ => panic!("Invalid input index {}", idx),
                }
            }
        }

        impl<$($D),+> SetupInputsSync for ($($crate::nodes::node_io::TypedInput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn setup_input_sync(&mut self, idx: NodeIOIndex, local: bool) {
                match idx {
                    $(
                        $idx => self.$idx.input.setup_input_sync(idx, local),
                    )+
                    _ => panic!("Invalid input index {}", idx),
                }
            }

           fn get_input_count(&self) -> NodeIOIndex {
                0 $(+ { let _ = &self.$idx; 1 })+ // Ensures correct summation
            }
        }
        )+
    };
}

/// **Macro to generate `SetupOutputs` implementations**
#[macro_export]
macro_rules! impl_setup_outputs {
    (() $(,)?) => {
        impl SetupOutputsSync for () {
            fn setup_output_sync(&mut self, _idx: NodeIOIndex, _local: bool) {}

            fn get_output_count(&self) -> NodeIOIndex {
                0 // No outputs
            }
        }
    };

    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait]
        impl<$($D),+> SetupOutputs for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            async fn setup_output(&mut self, idx: NodeIOIndex, local: bool) {
                match idx {
                    $(
                        $idx => self.$idx.output.setup_output(idx, local).await,
                    )+
                    _ => panic!("Invalid output index {}", idx),
                }
            }
        }

        impl<$($D),+> SetupOutputsSync for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn setup_output_sync(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => self.$idx.output.setup_output_sync(idx, local),
                    )+
                    _ => panic!("Invalid output index {}", idx),
                }
            }

            fn get_output_count(&self) -> NodeIOIndex {
        0 $(+ { let _ = &self.$idx; 1 })+ // Ensures correct summation
    }
        }
        )+
    };
}

macro_rules! impl_setup_inputs_sync {
    ($(($($D:ident),+)),+ $(,)?) => {
        $(
        impl<$($D),+> SetupInputsSync for ($(TypedInput<$D>,)+)
        where
            $($D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static),+
        {
            fn setup_input_sync(&mut self, idx: NodeIOIndex, local: bool) {
                match idx {
                    $(
                        $D => self.$D.input.setup_input_sync(idx, local),
                    )+
                    _ => (),
                }
            }
        }
        )+
    };
}

macro_rules! impl_setup_outputs_sync {
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        impl<$($D),+> SetupOutputsSync for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn setup_output_sync(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => self.$idx.output.setup_output_sync(idx, local),
                    )+
                    _ => panic!("Invalid output index {}", idx),
                }
            }

            fn get_output_count(&self) -> NodeIOIndex {
                0 $(+ { let _ = &self.$idx; 1 })+
            }
        }
        )+
    };
}

impl<I, O> SetupInputsSync for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs,
    O: SetupOutputsSync + SetupOutputs,
{
    fn setup_input_sync(&mut self, idx: NodeIOIndex, local: bool) {
        self.inputs.setup_input_sync(idx, local);
    }
    fn get_input_count(&self) -> NodeIOIndex {
        self.inputs.get_input_count()
    }
}

impl<I, O> SetupOutputsSync for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs,
    O: SetupOutputsSync + SetupOutputs,
{
    fn setup_output_sync(&mut self, idx: NodeIOIndex, local: bool) {
        self.outputs.setup_output_sync(idx, local);
    }

    fn get_output_count(&self) -> NodeIOIndex {
        self.outputs.get_output_count()
    }
}

impl<D> SetupOutputsSync for Output<D>
where
    D: 'static + Send + Sync + Debug + FromStr + Clone,
{
    #[cfg(not(target_arch = "wasm32"))]
    fn setup_output_sync(&mut self, idx: NodeIOIndex, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.setup_output(idx, local));
    }

    #[cfg(target_arch = "wasm32")]
    fn setup_output_sync(&mut self, _idx: NodeIOIndex, _local: bool) {
        panic!("setup_output_sync is not available in wasm");
    }

    fn get_output_count(&self) -> NodeIOIndex {
        1 // Single output
    }
}
impl<T> SetupInputsSync for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn setup_input_sync(&mut self, idx: NodeIOIndex, local: bool) {
        self.input.setup_input_sync(idx, local);
    }
    fn get_input_count(&self) -> NodeIOIndex {
        1 //single input
    }
}

pub trait SetupInputCommunicator<T: 'static + Send + Sync + Debug + FromStr + Clone> {
    fn get_input_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>>;
}

pub trait SetupOutputCommunicator<T: 'static + Send + Sync + Debug + FromStr + Clone> {
    fn get_output_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>>;
}

impl<T> SetupInputCommunicator<T> for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn get_input_communicator(&mut self, _idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.input.get_communicator_mut()
    }
}

impl<T, Rest> SetupInputCommunicator<T> for (TypedInput<T>, Rest)
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
    Rest: SetupInputCommunicator<T>,
{
    fn get_input_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        if idx == 0 {
            self.0.get_input_communicator(idx)
        } else {
            self.1.get_input_communicator(idx - 1)
        }
    }
}

impl<T> SetupOutputCommunicator<T> for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn get_output_communicator(&mut self, _idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.output.get_communicator_mut()
    }
}

impl<T, Rest> SetupOutputCommunicator<T> for (TypedOutput<T>, Rest)
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
    Rest: SetupOutputCommunicator<T>,
{
    fn get_output_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        if idx == 0 {
            self.0.get_output_communicator(idx)
        } else {
            self.1.get_output_communicator(idx - 1)
        }
    }
}

impl<I, O, T> SetupInputCommunicator<T> for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs + SetupInputCommunicator<T>,
    O: SetupOutputsSync + SetupOutputs,
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn get_input_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.inputs.get_input_communicator(idx)
    }
}

impl<I, O, T> SetupOutputCommunicator<T> for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs,
    O: SetupOutputsSync + SetupOutputs + SetupOutputCommunicator<T>,
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn get_output_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.outputs.get_output_communicator(idx)
    }
}

//===========================================================ASANY==========================================

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T> AsAny for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T> AsAny for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait AsAnyImpl {}

impl<T: 'static + AsAny> AsAnyImpl for T {}
impl<D> AsAny for Input<D>
where
    D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<I, O> AsAny for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs + 'static,
    O: SetupOutputsSync + SetupOutputs + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<D> AsAny for Output<D>
where
    D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl<T> TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub fn get_thread_communicator(&mut self) -> &mut ThreadCommunicator<T> {
        self.input
            .get_communicator_mut()
            .expect("ThreadCommunicator not found")
    }

    pub fn from_communicator(communicator: ThreadCommunicator<T>) -> Self {
        TypedInput {
            input: Input::from_communicator(communicator),
        }
    }
}

impl<T> TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub fn get_thread_communicator(&mut self) -> &mut ThreadCommunicator<T> {
        self.output
            .get_communicator_mut()
            .expect("ThreadCommunicator not found")
    }

    pub fn from_communicator(communicator: ThreadCommunicator<T>) -> Self {
        TypedOutput {
            output: Output::from_communicator(communicator),
        }
    }
}

//==========================================SETUP IO==================================================
//#[async_trait]

/// Helper trait for indexed tuple access of structures containing multiple tuples
pub trait SetupIO: Send + Sync + AsAny {
    fn get_input_count(&self) -> NodeIOIndex;
    fn get_output_count(&self) -> NodeIOIndex;
    fn get_input_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn has_ready_input(&self, idx: NodeIOIndex) -> bool;
}

impl<I, O> SetupIO for NodeIO<I, O>
where
    I: SetupInputsSync + SetupInputs + Send + Sync + TupleIO + 'static,
    O: SetupOutputsSync + SetupOutputs + Send + Sync + TupleIO + 'static,
{
    fn get_input_count(&self) -> NodeIOIndex {
        TupleIO::get_input_count(&self.inputs)
    }

    fn get_output_count(&self) -> NodeIOIndex {
        TupleIO::get_output_count(&self.outputs)
    }

    fn get_input_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut dyn Any> {
        TupleIO::get_input_communicator(&mut self.inputs, idx)
    }

    fn get_output_communicator(&mut self, idx: NodeIOIndex) -> Option<&mut dyn Any> {
        TupleIO::get_output_communicator(&mut self.outputs, idx)
    }

    fn has_ready_input(&self, idx: NodeIOIndex) -> bool {
        TupleIO::has_ready_input(&self.inputs, idx)
    }
}

//================================================TUPLE IO============================================

/// Helper trait for accessing tuple elements dynamically.
pub trait TupleIO: Send + Sync {
    fn get_output_count(&self) -> NodeIOIndex;
    fn get_input_count(&self) -> NodeIOIndex;
    fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn get_input_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn has_ready_input(&self, idx: NodeIOIndex) -> bool;
}

impl TupleIO for () {
    fn get_output_count(&self) -> NodeIOIndex {
        0
    }

    fn get_input_count(&self) -> NodeIOIndex {
        0
    }

    fn get_output_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
        None
    }

    fn get_input_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
        None
    }

    fn has_ready_input(&self, idx: NodeIOIndex) -> bool {
        true //empty inputs should always lead to execution
    }
}

// Macro to implement TupleIO for multiple inputs and outputs
macro_rules! impl_tuple_io {
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        // Implementing TupleIO for multiple outputs
        impl<$($D),+> TupleIO for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn get_output_count(&self) -> NodeIOIndex {
                0 $(+ { let _ = &self.$idx; 1 })+
            }

            fn get_input_count(&self) -> NodeIOIndex {
                0
            }

            fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any> {
                match index {
                    $(
                        $idx => Some(&mut self.$idx as &mut dyn Any),
                    )+
                    _ => None,
                }
            }

            fn get_input_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
                None
            }

            fn has_ready_input(&self, _idx: NodeIOIndex) -> bool {
                true // no inputs always lead to execution
            }
        }

        // Implementing TupleIO for multiple inputs
        impl<$($D),+> TupleIO for ($($crate::nodes::node_io::TypedInput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn get_output_count(&self) -> NodeIOIndex {
                0
            }

            fn get_input_count(&self) -> NodeIOIndex {
                0 $(+ { let _ = &self.$idx; 1 })+
            }

            fn get_output_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
                None
            }

            fn get_input_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any> {
                match index {
                    $(
                        $idx => Some(&mut self.$idx as &mut dyn Any),
                    )+
                    _ => None,
                }
            }

            fn has_ready_input(&self, idx: NodeIOIndex) -> bool {
                match idx {
                    $(
                        $idx => self.$idx.input.edge.has_data(),
                    )+
                    _ => true, // no inputs always need to execution
                }
            }
        }
        )+
    };
}

//================================TYPED INPUT/OUTPUT==============================
#[derive(Debug)]
pub struct TypedInput<I>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub input: Input<I>,
}

#[derive(Debug)]
pub struct TypedOutput<O>
where
    O: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub output: Output<O>,
}

// pub trait SplittableCommunicator: Send + Sync + AsAny {
//     fn split(
//         &mut self,
//         idx: NodeIOIndex,
//     ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>);
// }

// impl<T> SplittableCommunicator for TypedOutput<T>
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn split(
//         &mut self,
//         _idx: NodeIOIndex,
//     ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>) {
//         let comm = self
//             .output
//             .get_communicator_mut()
//             .expect("Expected ThreadCommunicator for split");

//         let send_half = comm.clone_send();
//         let recv_half = comm
//             .move_recv()
//             .expect("Failed to move receiver half from communicator");

//         let sender_output = Output::from_communicator(send_half);
//         let receiver_output = Output::from_communicator(recv_half);

//         (
//             Box::new(TypedOutput {
//                 output: sender_output,
//             }),
//             Box::new(TypedOutput {
//                 output: receiver_output,
//             }),
//         )
//     }
// }

// impl<T> SplittableCommunicator for TypedInput<T>
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn split(
//         &mut self,
//         _idx: NodeIOIndex,
//     ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>) {
//         if let Some(existing_comm) = self.input.get_communicator_mut() {
//             let send_half = existing_comm.clone_send();
//             let recv_half = existing_comm.move_recv().expect("Failed to move receiver");

//             (
//                 Box::new(TypedOutput::from_communicator(send_half))
//                     as Box<dyn SettableCommunicator>,
//                 Box::new(TypedOutput::from_communicator(recv_half))
//                     as Box<dyn SettableCommunicator>,
//             )
//         } else {
//             panic!("No communicator to split!");
//         }
//     }
// }

// pub trait CommunicatorGetter {
//     fn get_splittable(&mut self) -> Option<&mut dyn Splittable>;
// }

// impl<T> CommunicatorGetter for TypedOutput<T>
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn get_splittable(&mut self) -> Option<&mut dyn Splittable> {
//         self.output
//             .get_communicator_mut()
//             .map(|comm| comm as &mut dyn Splittable)
//     }
// }

pub trait SettableCommunicator: Send + Sync + AsAny {
    fn set_any_communicator(&mut self, communicator: Box<dyn Any + Send>);
}

impl<T> SettableCommunicator for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn set_any_communicator(&mut self, communicator: Box<dyn Any + Send>) {
        if let Ok(typed_comm) = communicator.downcast::<NodeCommunicator<T>>() {
            self.input.set_communicator(*typed_comm);
        } else {
            panic!("Failed to cast communicator to the expected type");
        }
    }
}

impl<T> SettableCommunicator for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn set_any_communicator(&mut self, communicator: Box<dyn Any + Send>) {
        if let Ok(typed_comm) = communicator.downcast::<NodeCommunicator<T>>() {
            self.output.set_communicator(*typed_comm);
        } else {
            panic!("Failed to cast communicator to the expected type");
        }
    }
}

impl<T> SetupOutputsSync for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn setup_output_sync(&mut self, idx: u128, local: bool) {
        self.output.setup_output_sync(idx, local);
    }

    fn get_output_count(&self) -> NodeIOIndex {
        1 // Each `TypedOutput<T>` represents a single output
    }
}

//===================================================================================

// Try to get mutable access to an Edge<D> by index
pub fn get_input_edge_mut<D: 'static>(
    io: &mut dyn SetupIO,
    idx: NodeIOIndex,
) -> Option<&mut Edge<D>>
where
    D: Clone,
    D: Debug,
    D: FromStr,
    D: Send + 'static,
{
    io.get_input_communicator(idx)
        .and_then(|any| any.downcast_mut::<Input<D>>())
        .map(|input| input.edge_mut())
}

//======================================TUPLE INVOCATIONS==========================================

impl_tuple_io!((0 D0));
impl_tuple_io!((0 D0, 1 D1));
impl_tuple_io!((0 D0, 1 D1, 2 D2));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7));

// **Implement Input and Output setup macros**
impl_setup_inputs!(());
impl_setup_outputs!(());

impl_setup_inputs!(
    (0 D0),
    (0 D0, 1 D1),
    (0 D0, 1 D1, 2 D2),
    (0 D0, 1 D1, 2 D2, 3 D3),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7)
);

impl_setup_outputs!(
    (0 D0),
    (0 D0, 1 D1),
    (0 D0, 1 D1, 2 D2),
    (0 D0, 1 D1, 2 D2, 3 D3),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6),
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7)
);
