use std::{
    any::Any,
    collections::HashMap,
    ops::Add,
    rc::Rc,
    sync::Arc
};

use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::{
    add::AddNode, basic::BasicNode, connection::ConnectError, debug::DebugNode, job::Context, Node,
};

#[derive(Clone, Debug)]
pub struct FlowType(pub Rc<dyn Any>);

// This implementation gives some control over which types should be
// addable throughout the entire flow. As of now only homogenious types
// allow addition.
// As the Properties of a Node can be any JSON value, the addition of
// such properties is limited to numbers (casted as float), lists and
// strings (both concatinated upon addition).
impl Add for FlowType {
    type Output = FlowType;

    fn add(self, rhs: Self) -> Self::Output {
        if let Some(lhs) = self.0.downcast_ref::<i64>() {
            if let Some(rhs) = rhs.0.downcast_ref::<i64>() {
                return FlowType(Rc::new(lhs+rhs));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<i32>() {
            if let Some(rhs) = rhs.0.downcast_ref::<i32>() {
                return FlowType(Rc::new(lhs+rhs));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<String>() {
            if let Some(rhs) = rhs.0.downcast_ref::<String>() {
                let mut res = lhs.clone();
                res.push_str(rhs);
                return FlowType(Rc::new(res));
            }
        }
        if let Some(lhs) = self.0.downcast_ref::<Value>() {
            if let Some(rhs) = rhs.0.downcast_ref::<Value>() {
                return match (lhs, rhs) {
                    (Value::Number(a), Value::Number(b)) => FlowType(Rc::new(a.as_f64().unwrap() + b.as_f64().unwrap())),
                    (Value::String(a), Value::String(b)) => {
                        let mut res = a.clone();
                        res.push_str(b);
                        FlowType(Rc::new(a.clone()))
                    },
                    (Value::Array(a), Value::Array(b)) => {
                        let mut res = a.clone();
                        res.append(b.to_owned().as_mut());
                        FlowType(Rc::new(a.clone()))
                    },
                    (a, b) => panic!("Addition of JSON values of type {:?} and {:?} is not supported.", a, b)
                }
            }
        }
        panic!("Addition not supported for type {:?} and {:?}.", self.type_id(), rhs.type_id());
    }
}

pub struct AppState {
    // For a yet TBD reason a HashMap of dyn types looses track of channel pointers.
    // As a workaround Nodes are resolved in a two step process and stored in a Vec.
    pub nodes: Vec<Box<dyn Node<FlowType, FlowType>>>,
    pub node_idc: HashMap<String, usize>,
    pub context: Arc<Context>,
}

impl AppState {
    pub fn new() -> AppState {
        AppState {
            nodes: Vec::new(),
            node_idc: HashMap::new(),
            context: Arc::new(Context {}),
        }
    }

    pub fn get_node(&mut self, name: &str) -> &dyn Node<FlowType, FlowType> {
        let index = *self.node_idc.get(name).unwrap();
        self.nodes[index].as_ref()
    }

    pub fn add_node(&mut self, name: &str, kind: String, props: Value) -> String {
        let node: Box<dyn Node<FlowType, FlowType>> = match kind.as_str() {
            "nodes.arithmetics.add" => {
                Box::new(AddNode::new(name, self.context.clone()))
            }
            "nodes.basic" => Box::new(BasicNode::new(
                name,
                self.context.clone(),
                FlowType(Rc::new(props)),
            )),
            "nodes.debug" => Box::new(DebugNode::new(name, self.context.clone())),
            value => panic!("Node kind `{}` is not yet supported.", value),
        };
        self.nodes.push(node);
        self.node_idc.insert(name.to_owned(), self.nodes.len() - 1);
        name.to_owned()
    }

    pub fn chain_at(
        &mut self,
        lhs: String,
        rhs: String,
        index: usize,
    ) -> Result<(), ConnectError<FlowType>> {
        let lhs_idx = self.node_idc.get(&lhs).unwrap().clone();
        let rhs_idx = self.node_idc.get(&rhs).unwrap().clone();
        let lhs_node = &self.nodes[lhs_idx];
        let rhs_node = &self.nodes[rhs_idx];
        lhs_node.chain(vec![rhs_node.input_at(index)?]);
        Ok(())
    }

    pub fn run(&mut self) {
        self.nodes.iter_mut().for_each(|job| job.on_init());
        self.nodes.iter_mut().for_each(|job| job.on_ready());
        // Capped at 100 here for testing purposes. TODO: change to infinite loop with stop condition.
        for _ in 0..100 {
            self.nodes.iter_mut().for_each(|job| job.on_handle());
        }
        self.nodes.iter_mut().for_each(|job| job.on_shutdown());
    }
}

#[derive(Deserialize)]
struct JsonNode {
    name: String,
    kind: String,
    props: Value,
    //inputs: Vec<Queue>,
    //outputs: Vec<Queue>,
}

#[derive(Deserialize)]
struct Queue {
    //ty: String,
}

#[derive(Deserialize)]
struct JsonEdge {
    input: String,
    index: usize,
    output: String,
}

#[derive(Deserialize)]
struct JsonData {
    nodes: Vec<JsonNode>,
    edges: Vec<JsonEdge>,
}

impl<'de> Deserialize<'de> for AppState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut app_state = AppState::new();
        let json_data: JsonData = JsonData::deserialize(deserializer)?;
        json_data.nodes.iter().for_each(|node| {
            app_state.add_node(&node.name, node.kind.clone(), node.props.to_owned());
        });
        json_data.edges.iter().for_each(|edge| {
            let _ = app_state.chain_at(edge.input.clone(), edge.output.clone(), edge.index);
        });

        Ok(app_state)
    }
}
