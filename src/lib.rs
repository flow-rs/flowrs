mod nodes;


use wasm_bindgen::prelude::wasm_bindgen;

use crate::add::AddNode;
use crate::debug::DebugNode;
use crate::job::Connectable;
use crate::job::Job;

pub use self::nodes::add;
pub use self::nodes::debug;
pub use self::nodes::job;

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
    let mut add1 = AddNode::new("Add1", 0, |i| i, |i| i);
    let mut add2 = AddNode::new("Add2", 0, |i| i, |i| i);
    let mut add3 = AddNode::new("Add3", 0, |i| i, |i| i);
    let debug: DebugNode<i32, i32> = DebugNode::new("PrintNode");
    // Init queues
    let _ = add1.input()[0].send(1);
    let _ = add1.input()[1].send(2);
    let _ = add2.input()[0].send(3);
    let _ = add2.input()[1].send(4);
    add1.connect(vec![add3.input()[0].clone(), debug.input()[0].clone()]);
    add2.connect(vec![add3.input()[1].clone(), debug.input()[0].clone()]);
    add3.connect(vec![debug.input()[0].clone()]);
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
