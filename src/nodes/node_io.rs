use crate::r#type::type_registry::{register_base_type, register_global};
use async_trait::async_trait;
use std::{fmt::Debug, str::FromStr};
use tokio::runtime::Runtime;

use super::connection::EdgeTrait;
use super::connection::Input;
use super::connection::Output;

/// The main I/O wrapper for all node implementations
pub struct NodeIO<I, O>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
    O: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub inputs: Input<I>,
    pub outputs: Output<O>,
}

impl<I, O> NodeIO<I, O>
where
    I: 'static + Send + Sync + Debug + FromStr + Clone,
    O: 'static + Send + Sync + Debug + FromStr + Clone,
{
    pub fn new(inputs: Input<I>, outputs: Output<O>) -> Self {
        Self::register_io_types();
        Self { inputs, outputs }
    }

    /// Register only base types `I` and `O`
    fn register_io_types() {
        tokio::spawn(async move {
            register_base_type::<I>().await;
            register_base_type::<O>().await;
        });
    }

    pub fn setup_input_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.inputs.setup_input(idx, local));
    }

    pub fn setup_output_sync(&mut self, idx: u128, local: bool) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(self.outputs.setup_output(idx, local));
    }
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
    () => {
        #[async_trait::async_trait]
        impl SetupInputs for () {
            async fn setup_input(&mut self, _idx: u128, _local: bool) {}
        }

        impl SetupInputsSync for () {
            fn setup_input_sync(&mut self, _idx: u128, _local: bool) {}
        }
    };

    // General case for multiple inputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupInputs for ($($crate::nodes::connection::Input<$D>,)+)
        where
            $(
                $D: Send + Debug + Clone + FromStr + 'static,  // ✅ Enforce `FromStr` per element
                $crate::nodes::connection::Input<$D>: $crate::nodes::connection::EdgeTrait<$D>,
            )+
        {
            async fn setup_input(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => {
                            self.$idx = if local {
                                <$crate::nodes::connection::Input<$D> as $crate::nodes::connection::EdgeTrait<$D>>::new_local()
                            } else {
                                <$crate::nodes::connection::Input<$D> as $crate::nodes::connection::EdgeTrait<$D>>::new_network().await
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
    () => {
        #[async_trait::async_trait]
        impl SetupOutputs for () {
            async fn setup_output(&mut self, _idx: u128, _local: bool) {}
        }

        impl SetupOutputsSync for () {
            fn setup_output_sync(&mut self, _idx: u128, _local: bool) {}
        }
    };

    // General case for multiple outputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupOutputs for ($($crate::nodes::connection::Output<$D>,)+)
        where
            $(
                $D: Send + Debug + Clone + FromStr + 'static,  // ✅ Enforce `FromStr` per element
                $crate::nodes::connection::Output<$D>: $crate::nodes::connection::EdgeTrait<$D>,
            )+
        {
            async fn setup_output(&mut self, idx: u128, local: bool) {
                match idx {
                    $(
                        $idx => {
                            self.$idx = if local {
                                <$crate::nodes::connection::Output<$D> as $crate::nodes::connection::EdgeTrait<$D>>::new_local()
                            } else {
                                <$crate::nodes::connection::Output<$D> as $crate::nodes::connection::EdgeTrait<$D>>::new_network().await
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

/// **Implement Input and Output setup macros**
impl_setup_inputs!(); // 0 inputs
impl_setup_outputs!(); // 0 outputs

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
