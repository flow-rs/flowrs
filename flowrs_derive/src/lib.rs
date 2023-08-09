mod connectable;

use proc_macro::TokenStream;

use connectable::impl_connectable_trait;

#[proc_macro_derive(Connectable, attributes(input, output))]
pub fn connectable_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();

    impl_connectable_trait(ast)
}
