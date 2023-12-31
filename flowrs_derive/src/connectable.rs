use core::panic;
use proc_macro::TokenStream;
use syn::{Arm, DataStruct, DeriveInput, Field, Ident, Type, WherePredicate};

pub fn impl_connectable_trait(ast: DeriveInput) -> TokenStream {
    let struct_ident = ast.clone().ident;
    let struct_ident_str = struct_ident.to_string();
    let inputs;
    let outputs;
    match ast.data {
        syn::Data::Struct(s) => {
            inputs = validate_struct_field(s.clone(), "Input", "input");
            outputs = validate_struct_field(s, "Output", "output");
        }
        _ => panic!("The derive(Connection) macro only works for structs."),
    };
    let input_len = inputs.len();
    let output_len = outputs.len();
    let inpu_arms: Vec<Arm> = inputs
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = &field.ident;
            let arm: TokenStream = quote::quote! {
                #index => Rc::new(self.#ident.clone()),
            }
            .into();
            let arm_ast: Arm = syn::parse(arm.clone()).unwrap();
            arm_ast
        })
        .collect::<Vec<Arm>>();
    let output_arms: Vec<Arm> = outputs
        .iter()
        .enumerate()
        .map(|(index, field)| {
            let ident = &field.ident;
            let arm: TokenStream = quote::quote! {
                #index => Rc::new(self.#ident.clone()),
            }
            .into();
            let arm_ast: Arm = syn::parse(arm.clone()).unwrap();
            arm_ast
        })
        .collect::<Vec<Arm>>();
    let (_, ty_generics, _) = ast.generics.split_for_impl();
    let mut generic_bounds = get_generic_bounds(inputs.clone());
    generic_bounds.append(&mut get_generic_bounds(outputs));
    quote::quote! {
        impl #ty_generics RuntimeConnectable for #struct_ident #ty_generics
        where
            #(#generic_bounds,)*
        {
            fn input_at(&self, index: usize) -> Rc<dyn Any> {
                match index {
                    #(#inpu_arms)*
                    _ => panic!("Index {} out of bounds for {} with input len {}.", index, #struct_ident_str, #input_len),
                }
            }

            fn output_at(&self, index: usize) -> Rc<dyn Any> {
                match index {
                    #(#output_arms)*
                    _ => panic!("Index {} out of bounds for {} with output len {}.", index, #struct_ident_str, #output_len),
                }
            }
        }
    }
    .into()
}

fn get_generic_bounds(fields: Vec<Field>) -> Vec<WherePredicate> {
    fields
        .iter()
        .map(|f| match &f.ty {
            Type::Path(path) => match &path.path.segments.first().unwrap().arguments {
                syn::PathArguments::AngleBracketed(angle) => match angle.args.first().unwrap() {
                    syn::GenericArgument::Type(generic) => match generic {
                        Type::Path(t_path) => {
                            let ident = t_path.path.segments.first().unwrap().ident.clone();
                            let cond: TokenStream = quote::quote! {
                                #ident: Clone + 'static
                            }
                            .into();
                            let cond_ast: WherePredicate = syn::parse(cond.clone()).unwrap();
                            cond_ast
                        }
                        _ => panic!("{}", generic_err(f.clone().ident.unwrap())),
                    },
                    _ => panic!("{}", generic_err(f.clone().ident.unwrap())),
                },
                _ => panic!("{}", generic_err(f.clone().ident.unwrap())),
            },
            _ => panic!("{}", generic_err(f.clone().ident.unwrap())),
        })
        .collect::<Vec<WherePredicate>>()
}

fn generic_err(field_name: Ident) -> String {
    format!(
        "Field: {} is not a valid Input/Output field for the flowrs_derive macro.",
        field_name
    )
}

fn validate_struct_field(strct: DataStruct, ty: &str, mcro: &str) -> Vec<Field> {
    strct
        .fields
        .into_iter()
        .filter_map(|f| {
            let type_valid = match f.clone().ty {
                Type::Path(p) => match p.path.segments.first() {
                    Some(segment) => segment.ident.to_string() == ty,
                    None => false,
                },
                _ => false,
            };
            let helper_macro_valid = match f.attrs.get(0) {
                Some(attr) => match &attr.meta {
                    syn::Meta::Path(p) => match p.segments.first() {
                        Some(segment) => segment.ident.to_string() == mcro,
                        None => false,
                    },
                    _ => false,
                },
                None => false,
            };
            match type_valid && helper_macro_valid {
                true => Some(f),
                false => None,
            }
        })
        .collect()
}
