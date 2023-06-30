mod nodes;

use std::collections::HashMap;
use std::sync::Arc;

use crate::add::AddNode;
use crate::debug::DebugNode;
use crate::job::Connectable;
use crate::job::Context;
use crate::job::Job;
use wasm_bindgen::prelude::wasm_bindgen;

pub use self::nodes::add;
pub use self::nodes::connection;
pub use self::nodes::debug;
pub use self::nodes::job;
pub use flow_derive::Connectable;

#[wasm_bindgen]
pub struct AppState {
    jobs: HashMap<u32, Box<dyn Job>>,
    context: Arc<Context>,
    counter: u32,
}

#[wasm_bindgen]
impl AppState {
    pub fn new() -> AppState {
        AppState {
            jobs: HashMap::new(),
            context: Arc::new(Context {}),
            counter: 0,
        }
    }

    pub fn add_node(&mut self, name: &str, kind: &str) -> u32 {
        let node: Box<dyn Job> = match kind {
            "add" => Box::new(AddNode::new(name, self.context.clone(), 0)),
            "debug" => Box::new(DebugNode::<i32, i32>::new(name, self.context.clone())),
            _ => panic!("Invalid node type"),
        };
        let _ = &self.jobs.insert(self.counter, node);
        self.counter += 1;
        &self.counter - 1
    }
}

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
    let _ = add1.send_at(0, 1);
    let _ = add1.send_at(1, 2);
    let _ = add2.send_at(0, 3);
    let _ = add2.send_at(1, 4);
    add1.chain(vec![add3.input().unwrap(), debug.input().unwrap()]);
    add2.chain(vec![add3.input_at(1).unwrap(), debug.input().unwrap()]);
    add3.chain(vec![debug.input().unwrap()]);
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
