mod nodes;

use std::sync::Arc;

use crate::add::AddNode;
use crate::debug::DebugNode;
use crate::job::Connectable;
use crate::job::Context;
use crate::job::Job;
use wasm_bindgen::prelude::wasm_bindgen;

pub use self::nodes::add;
pub use self::nodes::debug;
pub use self::nodes::job;
pub use flow_derive::Connectable;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

/// Executing an example flow with three add nodes and a debug node that
/// alerts the browser upon every cycle (capped at 100 cycles).
#[wasm_bindgen]
pub fn run_flow() {
    let context = Arc::new(Context {});
    let mut add1 = AddNode::new("Add1", context.clone(), 0);
    let mut add2 = AddNode::new("Add2", context.clone(), 0);
    let mut add3 = AddNode::new("Add3", context.clone(), 0);
    let debug: DebugNode<i32, i32> = DebugNode::new("PrintNode", context);
    // Init queues
    let _ = add1.send(1);
    let _ = add1.send_at(1, 2);
    let _ = add2.send(3);
    let _ = add2.send_at(1, 4);
    add1.chain(vec![add3.input()[0].clone(), debug.input()[0].clone()]);
    add2.chain(vec![add3.input()[1].clone(), debug.conn.input()[0].clone()]);
    add3.chain(vec![debug.conn.input()[0].clone()]);
    // Mocking a FIFO Queue considering two steps per addition and a buffer of
    // three for scheduling cycles where no item was present for processing.
    let mut jobs: Vec<Box<dyn Job>> = vec![
        Box::new(add1),
        Box::new(add2),
        Box::new(add3),
        Box::new(debug),
    ];
    for i in 0..100 {
        let len = jobs.len();
        jobs[i % len].handle();
        alert(&format!(
            "Job: {} with index {} is running.",
            jobs[i % len].name(),
            i % len
        ));
    }
}
