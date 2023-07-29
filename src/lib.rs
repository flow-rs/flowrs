mod flow;
mod nodes;

pub use self::nodes::add;
pub use self::nodes::basic;
pub use self::nodes::connection;
pub use self::nodes::debug;
pub use self::nodes::node;
pub use self::flow::app_state;
pub use flow_derive::Connectable;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}
