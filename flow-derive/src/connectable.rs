use core::panic;
use proc_macro::TokenStream;
use syn::{DeriveInput, Type};

pub fn impl_connectable_trait(ast: DeriveInput) -> TokenStream {
    // The struct name is required to know what Struct to impl the trait for.
    let struct_ident = ast.ident;
    let conn_field = match ast.data {
        syn::Data::Struct(s) => Some(s.fields.into_iter().filter(|f| {
            match &f.ty {
                Type::Path(p) => p
                    .path
                    .segments
                    .first()
                    .is_some_and(|f| f.ident.to_string() == "Connection"),
                _ => false,
            }
        })),
        _ => None,
    };
    // The generics can be applied as stated since any additional generics won't be used.
    let (_, ty_generics, where_clause) = ast.generics.split_for_impl();
    // The nested connection field is required to know what field to delegate the impl to.
    match conn_field.unwrap().next().clone() {
        None => panic!("Your Struct requires a field of type Connection<I, O> in order to derive from Connectable.\n
        Example:\n
        pub struct MyNode<I, O> {{conn: Connection<I, O>, ...}}\n
        Alternatively you can implement the Connectable trait manually without using this macro."),
        Some(field) => {
            let field_ident = field.ident;
            quote::quote! {
                use crate::nodes::connection::ConnectError;
                use super::connection::Connection;
                use std::sync::mpsc::Sender;
                impl #ty_generics Connectable<I, O> for #struct_ident #ty_generics #where_clause {
                    fn inputs(&self) -> &Vec<Sender<I>> {
                        &self.#field_ident.inputs()
                    }

                    fn output(&self) -> &Vec<Sender<O>> {
                        &self.#field_ident.output()
                    }

                    fn chain(&mut self, successors: Vec<std::sync::mpsc::Sender<O>>) {
                        self.#field_ident.chain(successors);
                    }

                    fn send_at(&self, index: usize, value: I) -> Result<(), ConnectError<I>> {
                        self.#field_ident.send_at(index, value)
                    }

                    fn send(&self, value: I) -> Result<(), ConnectError<I>> {
                        self.#field_ident.send(value)
                    }

                    fn input_at(&self, index: usize) -> Result<Sender<I>, ConnectError<I>> {
                        self.#field_ident.input_at(index)
                    }

                    fn input(&self) -> Result<Sender<I>, ConnectError<I>> {
                        self.#field_ident.input()
                    }

                    fn conn(&mut self) -> &mut Connection<I, O> {
                        &mut self.#field_ident
                    }

                    fn send_out(&self, elem: O) {
                        self.#field_ident.send_out(elem.clone());
                    }
                }
            }
            .into()
        }
    }
}