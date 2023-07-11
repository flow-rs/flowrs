mod connectable;
mod job;

use proc_macro::TokenStream;
use syn::ItemImpl;

use connectable::impl_connectable_trait;
use job::impl_job_trait;

#[proc_macro_derive(Connectable)]
pub fn connectable_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();

    impl_connectable_trait(ast)
}

/// A macro that converts multiple methods of the signature f: I -> () into a sequentially
/// structured handle method. Note that this macro shall only be used for implementations of the Job
/// trait and also requires the Connectable<I, O> trait to be satisfied.
/// 
/// # Example
/// 
/// ```
/// #[build_job]
/// impl<I, O> Job for AddNode<I, O>
/// where
///     I: Add<Output = O> + Clone,
///     O: Clone
/// {
///     fn handle_lhs(next_elem: I) {
///         self.state = Some(next_elem);
///     }
///
///     fn handle_rhs(next_elem: I) {
///         if let Some(input) = &self.state {
///             self.send_out(input.clone() + next_elem.clone());
///             self.state = None;
///         }
///     }
/// }
/// ```
/// 
/// In this example it is guaranteed that handle_lhs and handle_rhs
/// will wlays be executed sequentially to preserve synchronisazion
/// of given inputs. E.G. handle_lhs won't be executed twice before
/// handle_rhs was executed.
#[proc_macro_attribute]
pub fn build_job(_: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemImpl = syn::parse(item.clone()).unwrap();

    impl_job_trait(ast)
}
