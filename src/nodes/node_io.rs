use crate::connection::EdgeTrait;
use async_trait::async_trait;

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

impl<I, O> NodeIO<I, O> {
    pub fn new(inputs: I, outputs: O) -> Self {
        Self { inputs, outputs }
    }
}

/// This trait is used to set up the inputs and outputs with as little overhead as possible
#[async_trait]
pub trait SetupIO {
    async fn setup_input(&mut self, idx: u128, local: bool);
    async fn setup_output(&mut self, idx: u128, local: bool);
}

/// THis macro will create the SetupIO for tuples with the given input and output count
/// e.g. for the count of 3 generic variables, the call would be
/// impl_setup_io!((0 T0, 1 T1, 2 T2));
///
/// macro calls below
#[macro_export]
macro_rules! impl_setup_io {
    // Match for a single type with a generic parameter
    (($idx:tt $T:ident<$D:ident>)) => {
        #[async_trait::async_trait]
        impl<$D> SetupIO for $T<$D>
        where
            $T<$D>: crate::nodes::connection::EdgeTrait<$D>
                + Clone + Send + std::str::FromStr + std::fmt::Debug + 'static,
            $D: Clone + Send + std::str::FromStr + std::fmt::Debug + 'static,
        {
            async fn setup_input(&mut self, idx: u128, local: bool) {
                match idx {
                    $idx => {
                        *self = if local {
                            <$T<$D> as crate::nodes::connection::EdgeTrait<$D>>::new_local()
                        } else {
                            <$T<$D> as crate::nodes::connection::EdgeTrait<$D>>::new_network().await
                        };
                    }
                    _ => panic!("Invalid input index"),
                }
            }

            async fn setup_output(&mut self, idx: u128, local: bool) {
                match idx {
                    $idx => {
                        *self = if local {
                            <$T<$D> as crate::nodes::connection::EdgeTrait<$D>>::new_local()
                        } else {
                            <$T<$D> as crate::nodes::connection::EdgeTrait<$D>>::new_network().await
                        };
                    }
                    _ => panic!("Invalid output index"),
                }
            }
        }
    };

    // Match multiple types with generic parameters
    ($(($idx:tt $T:ident<$D:ident>)),+) => {
        $(
            impl_setup_io!(($idx $T<$D>));
        )+
    };
}

/// This macro call supports tuples of length 8.
/// For nodes which need longer variable lists, the macro can be imported and used in the node repository
impl_setup_io!(
    (0 Input<D0>),
    (1 Input<D1>),
    (2 Input<D2>),
    (0 Output<D0>),
    (1 Output<D1>),
    (2 Output<D2>),
    (3 Output<D3>)
);
