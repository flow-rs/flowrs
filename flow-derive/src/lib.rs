use proc_macro::TokenStream;
use syn::DeriveInput;

fn impl_connectable_trait(ast: DeriveInput) -> TokenStream {
    let ident = ast.ident;
    quote::quote! {
        impl<I, O> Connectable<I, O> for #ident<I, O> {
            fn input(&self) -> &Vec<Sender<I>> {
                &self.conn.input()
            }

            fn output(&self) -> &Vec<Sender<O>> {
                &self.conn.output()
            }

            fn chain(&mut self, successors: Vec<std::sync::mpsc::Sender<O>>) -> &Self {
                self.conn.chain(successors);
                self
            }

            fn send_at(&self, index: usize, value: I) {
                self.conn.send_at(index, value);
            }

            fn send(&self, value: I) {
                self.conn.send(value)
            }
        }
    }.into()
}

#[proc_macro_derive(Connectable)]
pub fn connectable_derive_macro(item: TokenStream) -> TokenStream {
    let ast = syn::parse(item).unwrap();

    impl_connectable_trait(ast)
}
