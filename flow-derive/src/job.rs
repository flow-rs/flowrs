use proc_macro::TokenStream;
use std::convert::identity;
use syn::{
    token::Brace, Block, FnArg, Ident, ImplItem, ImplItemFn, ItemImpl, Pat, ReturnType, Stmt,
};

/// This function takes an ItemImpl AST and merges multiple functions of the signature f: I -> () into
/// a single function of the signature f: I -> () while adding additional code to manage input queues
/// of a Connection field that must satisfy the Connectable<I, O> trait and be accessable from the impl
/// statement.
pub fn impl_job_trait(mut ast: ItemImpl) -> TokenStream {
    let fns: Vec<ImplItemFn> = get_funcs(&ast.clone());
    // The function blocks have to be extractd in order to create a handle method form it's business logic.
    let fns_match_signature = fns
        .clone()
        .iter()
        .all(|f| f.sig.inputs.len() == 1 && fn_input_type_is(f, "I") && fn_output_union(f));
    if !fns_match_signature {
        panic!("{}", sig_panic_msg());
    };
    let fn_blocks: Vec<Stmt> = fns
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let func = &fns[i].block;
            let block: TokenStream;
            // The above condition already validates that all functions have exactly one input parameter
            let input_ident;
            if let FnArg::Typed(ident) = &fns[i].sig.inputs[0] {
                if let Pat::Ident(p) = *ident.pat.to_owned() {
                    input_ident = p.ident;
                } else {
                    panic!("{}", sig_panic_msg());
                }
            } else {
                panic!("{}", sig_panic_msg());
            };
            let closing_stmt = get_closing_stmt(fns.len(), i);
            block = get_branch_stmt(func, i, input_ident, closing_stmt);
            syn::parse(block).unwrap()
        })
        .collect();
    let block_stmt = Block {
        stmts: fn_blocks,
        brace_token: Brace {
            ..Default::default()
        },
    };
    // The wrapping and repeated parsing for the handle method saves the overhead of
    // constructing the AST manually.
    let handle_fn_stmt: TokenStream = quote::quote! {
        fn handle(&mut self) {
            #block_stmt
            ()
        }
    }
    .into();
    let name_fn_stmt: TokenStream = quote::quote! {
        fn name(&self) -> &String {
            &self.name
    }
    }
    .into();
    let handle = syn::parse(handle_fn_stmt).unwrap();
    let name = syn::parse(name_fn_stmt).unwrap();
    // The entire Impl statement is ought to stay untouched except for it's function members.
    ast.items = vec![
        ImplItem::from(ImplItemFn::from(handle)),
        ImplItem::from(ImplItemFn::from(name)),
    ];
    let res: TokenStream = quote::quote! {#ast}.into();
    res
}

fn fn_input_type_is(f: &ImplItemFn, type_name: &str) -> bool {
    match &f.sig.inputs[0] {
        FnArg::Receiver(_) => false,
        FnArg::Typed(ref t) => match *t.ty.to_owned() {
            syn::Type::Path(p) => p.path.segments[0].ident.to_string() == type_name,
            _ => false,
        },
    }
}

fn fn_output_union(f: &ImplItemFn) -> bool {
    match f.sig.output {
        ReturnType::Default => true,
        ReturnType::Type(_, _) => false,
    }
}

fn get_branch_stmt(
    func: &Block,
    i: usize,
    input_ident_name: Ident,
    closing_stmt: Stmt,
) -> TokenStream {
    quote::quote! {
        // This is assuming that the Connectable<I, O> trait is met!
        if self.conn().state[#i].is_none() {
            self.conn().state[#i] = match self.conn().input.get(#i) {
                None => None,
                Some(i) => {
                    // Avoiding recv_timout since wasm can't access system time without JS bindings
                    match i.try_recv() {
                        Err(_) => return,
                        Ok(value) => Some(value)
                    }
                }
            };
            let mut #input_ident_name = self.conn().state[#i].clone().unwrap();
            #func;
            #closing_stmt
        }
    }
    .into()
}

fn get_closing_stmt(len: usize, index: usize) -> Stmt {
    let stmt;
    if index == len - 1 {
        stmt = quote::quote! {
            self.conn().state = (0..#len).enumerate().map(|(i, _)| None).collect();
        }
        .into();
    } else {
        stmt = quote::quote! {return;};
    }
    syn::parse(stmt.into()).unwrap()
}

fn get_funcs(ast: &ItemImpl) -> Vec<ImplItemFn> {
    ast.items
        .clone()
        .into_iter()
        .map(|f| match f {
            syn::ImplItem::Fn(f) => Some(f),
            _ => None,
        })
        .filter_map(identity)
        .collect()
}

fn sig_panic_msg() -> &'static str {
    "When using the job_macro, all functions of your implementation must match the form of `fn(my_elem: I) {{...}}` where I belongs to the corresponding Connectable<I, O> trait."
}
