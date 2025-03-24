use crate::comm::thread_communicator::{Splittable, ThreadCommunicator};
use crate::flow::flow_types::NodeIOIndex;
use async_trait::async_trait;
use std::any::Any;
use std::{fmt::Debug, str::FromStr};
use tokio::runtime::Runtime;

use super::connection::EdgeTrait;
use super::connection::Input;
use super::connection::Output;
use crate::comm::communication::{Communicator, NodeCommunicator};

/// The main I/O wrapper for all node implementationspub struct NodeIO<I, O>
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
        //Self::register_io_types();
        Self { inputs, outputs }
    }

    /// Register only base types `I` and `O`
    // fn register_io_types() {
    //     tokio::spawn(async move {
    //         I::register_types().await;
    //         O::register_types().await;
    //     });
    // }

    pub fn setup_input_sync(&mut self, idx: u128, local: bool) {
        let rt = Runtime::new().unwrap();
        rt.block_on(self.inputs.setup_input(idx, local));
    }

    pub fn setup_output_sync(&mut self, idx: u128, local: bool) {
        let rt = Runtime::new().unwrap();
        rt.block_on(self.outputs.setup_output(idx, local));
    }
}

pub struct TypedInput<I>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub input: Input<I>,
}

pub struct TypedOutput<O>
where
    O: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub output: Output<O>,
}

pub trait SplittableCommunicator: Send + Sync + AsAny {
    fn split(
        &mut self,
        idx: NodeIOIndex,
    ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>);
}

impl<T> SplittableCommunicator for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn split(
        &mut self,
        _idx: NodeIOIndex,
    ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>) {
        if let Some(existing_comm) = self.output.get_communicator_mut() {
            let send_half = existing_comm.clone_send();
            let recv_half = existing_comm.move_recv().expect("Failed to move receiver");

            (
                Box::new(TypedOutput {
                    output: Output::from_communicator(send_half),
                }),
                Box::new(TypedOutput {
                    output: Output::from_communicator(recv_half),
                }),
            )
        } else {
            panic!("No communicator to split!");
        }
    }
}

impl<T> SplittableCommunicator for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn split(
        &mut self,
        _idx: NodeIOIndex,
    ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>) {
        if let Some(existing_comm) = self.input.get_communicator_mut() {
            let send_half = existing_comm.clone_send();
            let recv_half = existing_comm.move_recv().expect("Failed to move receiver");

            (
                Box::new(TypedOutput::from_communicator(send_half))
                    as Box<dyn SettableCommunicator>,
                Box::new(TypedOutput::from_communicator(recv_half))
                    as Box<dyn SettableCommunicator>,
            )
        } else {
            panic!("No communicator to split!");
        }
    }
}

pub trait CommunicatorGetter {
    fn get_splittable(&mut self) -> Option<&mut dyn Splittable>;
}

impl<T> CommunicatorGetter for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn get_splittable(&mut self) -> Option<&mut dyn Splittable> {
        self.output
            .get_communicator_mut()
            .map(|comm| comm as &mut dyn Splittable)
    }
}

// impl<I, O> SplittableCommunicator for NodeIO<I, O>
// where
//     I: SetupInputsSync + SetupInputs + Send + Sync + 'static,
//     O: SetupOutputsSync + SetupOutputs + Send + Sync + 'static,
// {
//     fn split(
//         &mut self,
//         idx: NodeIOIndex,
//     ) -> (Box<dyn SettableCommunicator>, Box<dyn SettableCommunicator>) {
//         let output_count = self.outputs.get_output_count();

//         // Access the communicator via the getter
//         if let Some(output) = self.outputs.get_output(idx) {
//             if let Some(splittable) = output.get_splittable() {
//                 // Clone the sender part
//                 let send_half = splittable.clone_send_any();
//                 // Move the receiver part
//                 let recv_half = splittable.move_recv_any();

//                 // Wrap the split halves back into SettableCommunicator objects
//                 (
//                     Box::new(TypedOutput::from_any_communicator(send_half))
//                         as Box<dyn SettableCommunicator>,
//                     Box::new(TypedOutput::from_any_communicator(recv_half))
//                         as Box<dyn SettableCommunicator>,
//                 )
//             } else {
//                 panic!("Failed to access splittable communicator.");
//             }
//         } else {
//             panic!("No communicator found at index {}", idx);
//         }
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

/// **Helper functions to register individual types within tuples**
// async fn register_tuple_inputs<T>()
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     register_base_type::<T>().await;
// }

// async fn register_tuple_outputs<T>()
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     register_base_type::<T>().await;
// }

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

// /// **Recursive function to register each type in a tuple**
// async fn register_tuple<T>()
// where
//     T: 'static + Debug + Send + Sync,
// {
//     // This ensures that `T` is a valid type for registration.
//     register_global::<T, _>(|| panic!("Cannot create instance of generic type")).await;
// }

/// **Helper trait to register base types from a tuple**
// #[async_trait]
// pub trait RegisterBaseTypes {
//     async fn register_types();
// }

// /// **Base case for empty tuple (does nothing)**
// #[async_trait]
// impl RegisterBaseTypes for () {
//     async fn register_types() {}
// }

// impl<T> RegisterBaseTypes for TypedInput<T>
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     #[must_use]
//     #[allow(
//         elided_named_lifetimes,
//         clippy::type_complexity,
//         clippy::type_repetition_in_bounds
//     )]
//     fn register_types<'async_trait>() -> ::core::pin::Pin<
//         Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
//     > {
//         Box::pin(async move {
//             register_base_type::<T>().await;
//         })
//     }
// }

// impl<T> RegisterBaseTypes for TypedOutput<T>
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     #[must_use]
//     #[allow(
//         elided_named_lifetimes,
//         clippy::type_complexity,
//         clippy::type_repetition_in_bounds
//     )]
//     fn register_types<'async_trait>() -> ::core::pin::Pin<
//         Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
//     > {
//         Box::pin(async move {
//             register_base_type::<T>().await;
//         })
//     }
// }

/// **Traits for setting up inputs and outputs asynchronously**
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

// impl<I, O> SetupInputCommunicator for NodeIO<I, O>
// where
//     I: SetupInputsSync + SetupInputs + SetupInputCommunicator,
//     O: SetupOutputsSync + SetupOutputs,
// {
//     fn set_local_input_communicator(&mut self, idx: usize, comm: Box<dyn Any>) {
//         self.inputs.set_local_input_communicator(idx, comm);
//     }
// }

// impl<I, O> SetupOutputCommunicator for NodeIO<I, O>
// where
//     I: SetupInputsSync + SetupInputs,
//     O: SetupOutputsSync + SetupOutputs + SetupOutputCommunicator,
// {
//     fn set_local_output_communicator(&mut self, idx: usize, comm: Box<dyn Any>) {
//         self.outputs.set_local_output_communicator(idx, comm);
//     }
// }

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
    fn setup_input_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.setup_input(idx, local));
    }

    fn get_input_count(&self) -> NodeIOIndex {
        1 // Single input
    }
}

// impl<T, Rest> SetupInputsSync for (TypedInput<T>, Rest)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
//     Rest: SetupInputsSync,
// {
//     fn setup_input_sync(&mut self, idx: u128, local: bool) {
//         self.0.input.setup_input_sync(idx, local);
//         self.1.setup_input_sync(idx, local);
//     }
// }

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
    ($(($($D:ident),+)),+ $(,)?) => {
        $(
        impl<$($D),+> SetupOutputsSync for ($(TypedOutput<$D>,)+)
        where
            $($D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static),+
        {
            fn setup_output_sync(&mut self, idx: NodeIOIndex, local: bool) {
                match idx {
                    $(
                        $D => self.$D.input.setup_output_sync(idx, local),
                    )+
                    _ => (),
                }
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

// impl SetupInputsSync for () {
//     fn setup_input_sync(&mut self, _idx: u128, _local: bool) {
//         // No inputs, nothing to set up
//     }
// }

// impl SetupOutputsSync for () {
//     fn setup_output_sync(&mut self, _idx: u128, _local: bool) {
//         // No outputs, nothing to set up
//     }
// }

// Implement for single TypedOutput<T>
// impl<T> SetupOutputsSync for (TypedOutput<T>,)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn setup_output_sync(&mut self, idx: u128, local: bool) {
//         self.0.output.setup_output_sync(idx, local);
//     }
// }

// Recursive implementation for tuples of TypedOutput
// impl<T, Rest> SetupOutputsSync for (TypedOutput<T>, Rest)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
//     Rest: SetupOutputsSync,
// {
//     fn setup_output_sync(&mut self, idx: u128, local: bool) {
//         self.0.output.setup_output_sync(idx, local);
//         self.1.setup_output_sync(idx, local);
//     }
// }

impl<D> SetupOutputsSync for Output<D>
where
    D: 'static + Send + Sync + Debug + FromStr + Clone,
{
    fn setup_output_sync(&mut self, idx: NodeIOIndex, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.setup_output(idx, local));
    }
    fn get_output_count(&self) -> NodeIOIndex {
        1 // Single output
    }
}

// // Implement for single TypedInput<T>
// impl<T> SetupInputsSync for (TypedInput<T>,)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn setup_input_sync(&mut self, idx: u128, local: bool) {
//         self.0.input.setup_input_sync(idx, local);
//     }
// }

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
    T: 'static + Send + Sync + Debug + FromStr + Clone, // 🔥 Add these bounds
{
    fn get_input_communicator(&mut self, _idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.input.get_communicator_mut()
    }
}

impl<T, Rest> SetupInputCommunicator<T> for (TypedInput<T>, Rest)
where
    T: 'static + Send + Sync + Debug + FromStr + Clone, // 🔥 Add these bounds
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
    T: 'static + Send + Sync + Debug + FromStr + Clone, // 🔥 Add these bounds
{
    fn get_output_communicator(&mut self, _idx: NodeIOIndex) -> Option<&mut ThreadCommunicator<T>> {
        self.output.get_communicator_mut()
    }
}

impl<T, Rest> SetupOutputCommunicator<T> for (TypedOutput<T>, Rest)
where
    T: 'static + Send + Sync + Debug + FromStr + Clone, // 🔥 Add these bounds
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

// #[macro_export]
// macro_rules! impl_register_base_types {
//     // **Base case: Single element tuples**
//     ($(($D:ident)),+ $(,)?) => {
//         $(
//         #[async_trait::async_trait]
//         impl<$D> RegisterBaseTypes for (TypedOutput<$D>,)
//         where
//             $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
//         {
//             async fn register_types() {
//                 register_base_type::<$D>().await;
//             }
//         }

//         #[async_trait::async_trait]
//         impl<$D> RegisterBaseTypes for (TypedInput<$D>,)
//         where
//             $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
//         {
//             async fn register_types() {
//                 register_base_type::<$D>().await;
//             }
//         }
//         )+
//     };

//     // **Recursive case: Multiple elements**
//     ($(($($D:ident),+)),+ $(,)?) => {
//         $(
//         #[async_trait::async_trait]
//         impl<$($D),+> RegisterBaseTypes for ($(TypedOutput<$D>,)+)
//         where
//             $(
//                 $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static
//             ),+
//         {
//             async fn register_types() {
//                 $(
//                     register_base_type::<$D>().await;
//                 )+
//             }
//         }

//         #[async_trait::async_trait]
//         impl<$($D),+> RegisterBaseTypes for ($(TypedInput<$D>,)+)
//         where
//             $(
//                 $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static
//             ),+
//         {
//             async fn register_types() {
//                 $(
//                     register_base_type::<$D>().await;
//                 )+
//             }
//         }
//         )+
//     };
// }

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

pub trait SetupIO: Send + Sync + AsAny {
    fn get_input_count(&self) -> NodeIOIndex;
    fn get_output_count(&self) -> NodeIOIndex;
    fn get_input_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
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
}

/// Helper trait for accessing tuple elements dynamically.
pub trait TupleIO: Send + Sync {
    fn get_output_count(&self) -> NodeIOIndex;
    fn get_input_count(&self) -> NodeIOIndex;
    fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
    fn get_input_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any>;
}

// // Implementing TupleIO for a single output
// impl<T> TupleIO for (TypedOutput<T>,)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn get_output_count(&self) -> NodeIOIndex {
//         1
//     }

//     fn get_input_count(&self) -> NodeIOIndex {
//         0
//     }

//     fn get_output_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
//         Some(&mut self.0.output as &mut dyn Any)
//     }

//     fn get_input_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
//         None
//     }
// }

// // Implementing TupleIO for a single input
// impl<T> TupleIO for (TypedInput<T>,)
// where
//     T: 'static + Send + Sync + Debug + FromStr + Clone,
// {
//     fn get_output_count(&self) -> NodeIOIndex {
//         0
//     }

//     fn get_input_count(&self) -> NodeIOIndex {
//         1
//     }

//     fn get_output_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
//         None
//     }

//     fn get_input_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
//         Some(&mut self.0.input as &mut dyn Any)
//     }
// }

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
                        $idx => Some(&mut self.$idx.output as &mut dyn Any),
                    )+
                    _ => None,
                }
            }

            fn get_input_communicator(&mut self, _index: NodeIOIndex) -> Option<&mut dyn Any> {
                None
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
                        $idx => Some(&mut self.$idx.input as &mut dyn Any),
                    )+
                    _ => None,
                }
            }
        }
        )+
    };
}

// /// Macro to implement `TupleIO` for various tuple sizes.
// macro_rules! impl_tuple_io {
//     ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
//         $(
//         impl<$($D),+> TupleIO for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
//         where
//             $(
//                 $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
//             ),+
//         {
//             fn get_count(&self) -> NodeIOIndex {
//                 0 $(+ { let _ = &self.$idx; 1 })+ // Count elements in the tuple
//             }

//             fn get_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any> {
//                 match index {
//                     $(
//                         $idx => Some(&mut self.$idx.output as &mut dyn Any),
//                     )+
//                     _ => None,
//                 }
//             }
//         }
//         )+
//     };
// }

impl_tuple_io!((0 D0));
impl_tuple_io!((0 D0, 1 D1));
impl_tuple_io!((0 D0, 1 D1, 2 D2));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6));
impl_tuple_io!((0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7));

// macro_rules! impl_setup_io {
//     ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
//         $(
//         impl<$($D),+> SetupIO for ($($crate::nodes::node_io::TypedOutput<$D>,)+)
//         where
//             $(
//                 $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
//             ),+
//         {
//             fn get_input_count(&self) -> NodeIOIndex {
//                 0 // No inputs in outputs tuple
//             }

//             fn get_output_count(&self) -> NodeIOIndex {
//                 0 $(+ { let _ = &(self.$idx); 1 })+ // Correct summation of output elements
//             }

//             fn get_output_communicator(&mut self, index: NodeIOIndex) -> Option<&mut dyn Any> {
//                 match index {
//                     $(
//                         $idx => Some(&mut (self.$idx).output as &mut dyn Any),
//                     )+
//                     _ => None,
//                 }
//             }
//         }
//         )+
//     };
// }

// impl_setup_io!((0 D0));
// impl_setup_io!((0 D0), (1 D1));
// impl_setup_io!((0 D0), (1 D1), (2 D2));
// impl_setup_io!((0 D0), (1 D1), (2 D2), (3 D3));
// impl_setup_io!((0 D0), (1 D1), (2 D2), (3 D3), (4 D4));
// impl_setup_io!((0 D0), (1 D1), (2 D2), (3 D3), (4 D4), (5 D5));
// impl_setup_io!((0 D0), (1 D1), (2 D2), (3 D3), (4 D4), (5 D5), (6 D6));
// impl_setup_io!((0 D0), (1 D1), (2 D2), (3 D3), (4 D4), (5 D5), (6 D6), (7 D7));

/// **Implement Input and Output setup macros**
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

// impl_register_base_types!(
//     (D0),
//     (D0, D1),
//     (D0, D1, D2),
//     (D0, D1, D2, D3),
//     (D0, D1, D2, D3, D4),
//     (D0, D1, D2, D3, D4, D5),
//     (D0, D1, D2, D3, D4, D5, D6),
//     (D0, D1, D2, D3, D4, D5, D6, D7)
// );
