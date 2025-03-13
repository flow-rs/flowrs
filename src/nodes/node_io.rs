use crate::r#type::type_registry::{register_base_type, register_global};
use async_trait::async_trait;
use std::{fmt::Debug, str::FromStr};
use tokio::runtime::Runtime;

use super::connection::EdgeTrait;
use super::connection::Input;
use super::connection::Output;

/// The main I/O wrapper for all node implementationspub struct NodeIO<I, O>
pub struct NodeIO<I, O>
where
    I: SetupInputs,  // Change `FromStr + Clone` to `SetupInputs`
    O: SetupOutputs, // Change `FromStr + Clone` to `SetupOutputs`
{
    pub inputs: I,
    pub outputs: O,
}

impl<I, O> NodeIO<I, O>
where
    I: SetupInputs + RegisterBaseTypes + Send + Sync, // Ensure it implements `RegisterBaseTypes`
    O: SetupOutputs + RegisterBaseTypes + Send + Sync, // Ensure it implements `RegisterBaseTypes`
{
    pub fn new(inputs: I, outputs: O) -> Self {
        Self::register_io_types();
        Self { inputs, outputs }
    }

    /// Register only base types `I` and `O`
    fn register_io_types() {
        tokio::spawn(async move {
            I::register_types().await;
            O::register_types().await;
        });
    }

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
/// **Helper functions to register individual types within tuples**
async fn register_tuple_inputs<T>()
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    register_base_type::<T>().await;
}

async fn register_tuple_outputs<T>()
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    register_base_type::<T>().await;
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
#[async_trait]
pub trait RegisterBaseTypes {
    async fn register_types();
}

/// **Base case for empty tuple (does nothing)**
#[async_trait]
impl RegisterBaseTypes for () {
    async fn register_types() {}
}

impl<T> RegisterBaseTypes for TypedInput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    fn register_types<'async_trait>() -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    > {
        Box::pin(async move {
            register_base_type::<T>().await;
        })
    }
}

impl<T> RegisterBaseTypes for TypedOutput<T>
where
    T: 'static + Send + Sync + Debug + FromStr + Clone,
{
    #[must_use]
    #[allow(
        elided_named_lifetimes,
        clippy::type_complexity,
        clippy::type_repetition_in_bounds
    )]
    fn register_types<'async_trait>() -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    > {
        Box::pin(async move {
            register_base_type::<T>().await;
        })
    }
}

/// **Traits for setting up inputs and outputs asynchronously**
#[async_trait]
pub trait SetupInputs {
    async fn setup_input(&mut self, idx: u128, local: bool);
}

#[async_trait]
pub trait SetupOutputs {
    async fn setup_output(&mut self, idx: u128, local: bool);
}

/// **Synchronous wrapper traits**
pub trait SetupInputsSync {
    fn setup_input_sync(&mut self, idx: u128, local: bool);
}

pub trait SetupOutputsSync {
    fn setup_output_sync(&mut self, idx: u128, local: bool);
}

#[async_trait]
impl<I> SetupInputs for Input<I>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
{
    async fn setup_input(&mut self, idx: u128, local: bool) {
        *self = if local {
            Input::new_local()
        } else {
            Input::new_network().await
        };
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

/// **Macro to generate `SetupInputs` implementations**
#[macro_export]
macro_rules! impl_setup_inputs {
    // Special case for zero inputs
    (() $(,)?) => {
        #[async_trait::async_trait]
        impl SetupInputs for () {
            async fn setup_input(&mut self, _idx: u128, _local: bool) {}
        }
    };

    // General case for multiple inputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupInputs for ($($crate::nodes::node_io::TypedInput<$D>,)+) // <-- Ensure full path
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            async fn setup_input(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => {
                            self.$idx.input = if local {
                                <$crate::nodes::connection::Input<$D>>::new_local()
                            } else {
                                <$crate::nodes::connection::Input<$D>>::new_network().await
                            };
                        }
                    )+
                    _ => (),
                }
            }
        }
        )+
    };
}

/// **Macro to generate `SetupOutputs` implementations**
#[macro_export]
macro_rules! impl_setup_outputs {
    // Special case for zero outputs
    (() $(,)?) => {
        #[async_trait::async_trait]
        impl SetupOutputs for () {
            async fn setup_output(&mut self, _idx: u128, _local: bool) {}
        }
    };

    // General case for multiple outputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupOutputs for ($($crate::nodes::node_io::TypedOutput<$D>,)+) // <-- Ensure full path
        where
            $(
                $D: Clone + Send + Sync + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            async fn setup_output(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => {
                            self.$idx.output = if local {
                                <$crate::nodes::connection::Output<$D>>::new_local()
                            } else {
                                <$crate::nodes::connection::Output<$D>>::new_network().await
                            };
                        }
                    )+
                    _ => (),
                }
            }
        }
        )+
    };
}

#[macro_export]
macro_rules! impl_register_base_types {
    // **Base case: Single element tuples**
    ($(($D:ident)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$D> RegisterBaseTypes for (TypedOutput<$D>,)
        where
            $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
        {
            async fn register_types() {
                register_base_type::<$D>().await;
            }
        }

        #[async_trait::async_trait]
        impl<$D> RegisterBaseTypes for (TypedInput<$D>,)
        where
            $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static,
        {
            async fn register_types() {
                register_base_type::<$D>().await;
            }
        }
        )+
    };

    // **Recursive case: Multiple elements**
    ($(($($D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> RegisterBaseTypes for ($(TypedOutput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static
            ),+
        {
            async fn register_types() {
                $(
                    register_base_type::<$D>().await;
                )+
            }
        }

        #[async_trait::async_trait]
        impl<$($D),+> RegisterBaseTypes for ($(TypedInput<$D>,)+)
        where
            $(
                $D: Clone + Send + Sync + std::fmt::Debug + std::str::FromStr + 'static
            ),+
        {
            async fn register_types() {
                $(
                    register_base_type::<$D>().await;
                )+
            }
        }
        )+
    };
}
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

impl_register_base_types!(
    (D0),
    (D0, D1),
    (D0, D1, D2),
    (D0, D1, D2, D3),
    (D0, D1, D2, D3, D4),
    (D0, D1, D2, D3, D4, D5),
    (D0, D1, D2, D3, D4, D5, D6),
    (D0, D1, D2, D3, D4, D5, D6, D7)
);
