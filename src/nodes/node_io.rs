use crate::connection::EdgeTrait;
use async_trait::async_trait;
use tokio::runtime::Runtime;

use super::connection::{Input, Output};
/// This mod will allow node implementations to take on inputs and outputs of arbitrary length and generic types
/// The macro allows node types to define Inputs and Outputs as Tupels. Example:
/// pub struct AddNode<I1, I2, O>
/// where
///    I1: Clone + Send + 'static,
///    I2: Clone + Send + 'static,
///    O: Clone + Send + 'static,
///{
///    io: NodeIO<(Input<I1>, Input<I2>), (Output<O>,)>,
///}
/// IMPORTANT: The macro expands the I/O count. For unique combinations of I/O counts, a new macro call must be placed

/// The main IO wrapper for all node implementations
pub struct NodeIO<I, O> {
    pub inputs: I,
    pub outputs: O,
}

impl<I, O> NodeIO<I, O>
where
    I: SetupInputs,
    O: SetupOutputs,
{
    pub fn new(inputs: I, outputs: O) -> Self {
        Self { inputs, outputs }
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

/// This trait is used to set up the inputs and outputs with as little overhead as possible
#[async_trait]
pub trait SetupInputs {
    async fn setup_input(&mut self, idx: u128, local: bool);
}
#[async_trait]
pub trait SetupOutputs {
    async fn setup_output(&mut self, idx: u128, local: bool);
}

pub trait SetupInputsSync {
    fn setup_input_sync(&mut self, idx: u128, local: bool);
}

pub trait SetupOutputsSync {
    fn setup_output_sync(&mut self, idx: u128, local: bool);
}

/// THis macro will create the SetupIO for tuples with the given input and output count
/// e.g. for the count of 3 generic variables, the call would be
/// impl_setup_io!((0 T0, 1 T1, 2 T2));
///
/// macro calls below
#[macro_export]
macro_rules! impl_setup_inputs {
    // Special case for zero inputs
    () => {
        #[async_trait::async_trait]
        impl SetupInputs for () {
            async fn setup_input(&mut self, _idx: u128, _local: bool) {
                // No-op for zero inputs
            }
        }

        impl SetupInputsSync for () {
            fn setup_input_sync(&mut self, _idx: u128, _local: bool) {
                // No-op for zero inputs
            }
        }
    };

    // General case for multiple inputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupInputs for ($($crate::nodes::connection::Input<$D>,)+)
        where
            $(
                $crate::nodes::connection::Input<$D>: $crate::nodes::connection::EdgeTrait<$D>,
                $D: Clone + Send + std::str::FromStr + std::fmt::Debug + 'static
            ),+
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

        impl<$($D),+> SetupInputsSync for ($($crate::nodes::connection::Input<$D>,)+)
        where
            $(
                $crate::nodes::connection::Input<$D>: $crate::nodes::connection::EdgeTrait<$D>,
                $D: Clone + Send + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn setup_input_sync(&mut self, idx: u128, local: bool) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.setup_input(idx, local));
            }
        }
        )+
    };
}

#[macro_export]
macro_rules! impl_setup_outputs {
    // Special case for zero outputs
    () => {
        #[async_trait::async_trait]
        impl SetupOutputs for () {
            async fn setup_output(&mut self, _idx: u128, _local: bool) {
                // No-op for zero outputs
            }
        }

        impl SetupOutputsSync for () {
            fn setup_output_sync(&mut self, _idx: u128, _local: bool) {
                // No-op for zero outputs
            }
        }
    };

    // General case for multiple outputs
    ($(($($idx:tt $D:ident),+)),+ $(,)?) => {
        $(
        #[async_trait::async_trait]
        impl<$($D),+> SetupOutputs for ($($crate::nodes::connection::Output<$D>,)+)
        where
            $(
                $crate::nodes::connection::Output<$D>: $crate::nodes::connection::EdgeTrait<$D>,
                $D: Clone + Send + std::str::FromStr + std::fmt::Debug + 'static
            ),+
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

        impl<$($D),+> SetupOutputsSync for ($($crate::nodes::connection::Output<$D>,)+)
        where
            $(
                $crate::nodes::connection::Output<$D>: $crate::nodes::connection::EdgeTrait<$D>,
                $D: Clone + Send + std::str::FromStr + std::fmt::Debug + 'static
            ),+
        {
            fn setup_output_sync(&mut self, idx: u128, local: bool) {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(self.setup_output(idx, local));
            }
        }
        )+
    };
}

impl_setup_inputs!(); // 0 inputs

impl_setup_inputs!(
    (0 D0),                                             // 1 input
    (0 D0, 1 D1),                                       // 2 inputs
    (0 D0, 1 D1, 2 D2),                                 // 3 inputs
    (0 D0, 1 D1, 2 D2, 3 D3),                           // 4 inputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4),                     // 5 inputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5),               // 6 inputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6),         // 7 inputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7)    // 8 inputs
);

impl_setup_outputs!(); // 0 outputs
impl_setup_outputs!(
    (0 D0),                                             // 1 output
    (0 D0, 1 D1),                                       // 2 outputs
    (0 D0, 1 D1, 2 D2),                                 // 3 outputs
    (0 D0, 1 D1, 2 D2, 3 D3),                           // 4 outputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4),                     // 5 outputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5),               // 6 outputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6),         // 7 outputs
    (0 D0, 1 D1, 2 D2, 3 D3, 4 D4, 5 D5, 6 D6, 7 D7)    // 8 outputs
);
