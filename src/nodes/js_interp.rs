use std::sync::Arc;

use boa_engine::JsValue;
use boa_engine::property::Attribute;
use flow_derive::build_job;
use serde::Deserialize;

use crate::job::{Context, Job};
use crate::Connectable;

#[derive(Connectable, Deserialize)]
pub struct DebugNode<I, O>
where
    I: Sized + Clone,
    O: Sized + Clone,
{
    conn: Connection<I, O>,
    _context: Arc<Context>,
    _state: String,
    js_code: String,
    name: String,
}

impl<I, O> DebugNode<I, O>
where
    I: Clone,
    O: Clone,
{
    pub fn new(name: &str, context: Arc<Context>, js_code: String) -> Self {
        let conn = Connection::new(1);
        Self {
            _state: "".into(),
            conn,
            js_code,
            name: name.into(),
            _context: context,
        }
    }
}

#[build_job]
impl<I, O> Job for DebugNode<I, O>
where
    I: Clone + ToString,
    O: Clone, JsValue: From<I>
{
    fn handle(next_elem: I) {

        let mut js_ctx = boa_engine::Context::default();
        // Ignoring error for now since it only indicates whether the property was present beforehand.
        let _ = js_ctx.register_global_property("state", boa_engine::JsValue::Undefined, Attribute::WRITABLE);
        let _ = js_ctx.register_global_property("next_elem", next_elem, Attribute::WRITABLE);

        match js_ctx.eval(boa_engine::Source::from_bytes(&self.js_code)) {
            Ok(res) => {
                println!(
                    "{}",
                    res.to_string(&mut js_ctx).unwrap().to_std_string_escaped()
                );
            }
            Err(e) => {
                // Pretty print the error
                eprintln!("Uncaught {e}");
            }
        };
    }
}
