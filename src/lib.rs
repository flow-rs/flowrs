mod nodes;
mod flow;

pub use self::nodes::add;
pub use self::nodes::basic;
pub use self::nodes::connection;
pub use self::nodes::job;
pub use self::nodes::debug;
pub use self::nodes::job::Node;
pub use flow::app_state;
pub use flow_derive::Connectable;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}