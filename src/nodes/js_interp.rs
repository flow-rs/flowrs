use std::sync::Arc;

use boa_engine::JsValue;
use boa_engine::property::Attribute;
use flow_derive::build_job;
use serde::Deserialize;

use crate::job::{Context, Job};

#[derive(Deserialize)]
pub struct DebugNode
{
    _context: Arc<Context>,
    _state: String,
    js_code: String,
    name: String,
}

impl DebugNode
{
    pub fn new(name: &str, context: Arc<Context>, js_code: String) -> Self {
        Self {
            _state: "".into(),
            js_code,
            name: name.into(),
            _context: context,
        }
    }
}
